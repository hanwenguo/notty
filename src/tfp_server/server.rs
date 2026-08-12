use std::collections::HashMap;
use std::path::{Component, Path, PathBuf};

use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use std::sync::Arc;
use typst_kit::fonts::FontStore;
use typst_syntax::{FileId, RootedPath, VirtualPath, VirtualRoot};

use crate::compiler::world::LibraryWorld;
use crate::tfp_server::project::WeibianProject;
use crate::tfp_server::protocol::{PROTOCOL_VERSION, RpcError, server_target, typst_version};
use crate::tfp_server::render::{RenderMathParams, render_math};
use crate::tfp_server::source::{OpenSource, TextChange};
use crate::tfp_server::world::{
    DocumentConfig, create_project_world, load_fonts, prepare_project_world,
};

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct InitializeParams {
    protocol_version: u32,
    #[serde(default)]
    root: Option<PathBuf>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct OpenParams {
    path: String,
    text: String,
    version: u64,
    #[serde(default)]
    config: DocumentConfig,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct ChangeParams {
    path: String,
    version: u64,
    changes: Vec<TextChange>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct FullSyncParams {
    path: String,
    text: String,
    version: u64,
}

#[derive(Debug, Deserialize)]
struct PathParams {
    path: String,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct VersionedPathParams {
    path: String,
    version: u64,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct EditResult {
    version: u64,
    dirty_start_line: usize,
    dirty_end_line: usize,
}

pub struct Server {
    root: PathBuf,
    initialized: bool,
    shutdown: bool,
    documents: HashMap<String, OpenSource>,
    configs: HashMap<String, DocumentConfig>,
    worlds: HashMap<String, LibraryWorld>,
    fonts: HashMap<String, Arc<FontStore>>,
    project: WeibianProject,
}

impl Server {
    pub fn new(root: PathBuf, project: WeibianProject) -> Result<Self, String> {
        let root = root
            .canonicalize()
            .map_err(|error| format!("cannot canonicalize project root: {error}"))?;
        Ok(Self {
            root,
            initialized: false,
            shutdown: false,
            documents: HashMap::new(),
            configs: HashMap::new(),
            worlds: HashMap::new(),
            fonts: HashMap::new(),
            project,
        })
    }

    pub fn handle(&mut self, method: &str, params: Value) -> Result<Value, RpcError> {
        if method != "initialize" && !self.initialized {
            return Err(invalid_request("server has not been initialized"));
        }
        if self.shutdown && method != "exit" {
            return Err(invalid_request("server is shutting down"));
        }

        match method {
            "initialize" => self.initialize(parse(params)?),
            "textDocument/didOpen" => self.did_open(parse(params)?),
            "textDocument/didChange" => self.did_change(parse(params)?),
            "textDocument/didClose" => self.did_close(parse(params)?),
            "tfp/fullSync" => self.full_sync(parse(params)?),
            "tfp/equations" => self.equations(parse(params)?),
            "tfp/status" => Ok(self.status()),
            "shutdown" => {
                self.shutdown = true;
                Ok(Value::Null)
            }
            "exit" => Ok(Value::Null),
            "tfp/renderMath" => self.render_math(parse(params)?),
            _ => Err(RpcError {
                code: -32601,
                message: format!("unknown method {method}"),
                data: None,
            }),
        }
    }

    pub fn should_exit(&self) -> bool {
        self.shutdown
    }

    #[cfg(test)]
    fn document_version(&self, path: &str) -> Option<u64> {
        self.documents.get(path).map(|document| document.version)
    }

    fn prepare_world(&mut self, path: &str) -> Result<(), RpcError> {
        if !self.documents.contains_key(path) {
            return Err(invalid_params(format!("document is not open: {path}")));
        }
        let overlays: HashMap<_, _> = self
            .documents
            .values()
            .map(|open| (open.source.id(), open.source.clone()))
            .collect();
        let mut overlays = overlays;
        let entrypoint = self
            .project
            .entrypoint_source(self.documents.keys().map(String::as_str))
            .map_err(|message| RpcError {
                code: -32002,
                message: message.to_string(),
                data: None,
            })?;
        let main = entrypoint.id();
        overlays.insert(main, entrypoint);
        let config = self.configs.get(path).cloned().unwrap_or_default();
        if let Some(world) = self.worlds.get_mut(path) {
            prepare_project_world(world, main, overlays, config.target).map_err(|message| {
                RpcError {
                    code: -32002,
                    message,
                    data: None,
                }
            })?;
            return Ok(());
        }
        let font_key = format!(
            "{}:{}:{:?}",
            config.ignore_system_fonts, config.ignore_embedded_fonts, config.font_paths
        );
        let fonts = self
            .fonts
            .entry(font_key)
            .or_insert_with(|| load_fonts(&config))
            .clone();
        let world = create_project_world(self.root.clone(), main, overlays, &config, fonts)
            .map_err(|message| RpcError {
                code: -32002,
                message,
                data: None,
            })?;
        self.worlds.insert(path.into(), world);
        Ok(())
    }

    fn initialize(&mut self, params: InitializeParams) -> Result<Value, RpcError> {
        if params.protocol_version != PROTOCOL_VERSION {
            return Err(RpcError {
                code: -32001,
                message: format!(
                    "protocol mismatch: client {}, server {}",
                    params.protocol_version, PROTOCOL_VERSION
                ),
                data: Some(json!({"serverProtocolVersion": PROTOCOL_VERSION})),
            });
        }
        if let Some(root) = params.root {
            let root = root.canonicalize().map_err(|error| {
                invalid_params(format!("cannot canonicalize initialize root: {error}"))
            })?;
            if root != self.root {
                return Err(invalid_params("initialize root differs from process root"));
            }
        }
        self.initialized = true;
        Ok(json!({
            "protocolVersion": PROTOCOL_VERSION,
            "serverVersion": env!("CARGO_PKG_VERSION"),
            "typstVersion": typst_version(),
            "target": server_target(),
            "capabilities": {
                "incrementalSync": true,
                "fullDocumentMath": true,
                "exactBaseline": true,
                "transparentSvg": true,
                "unsavedImports": true,
                "equationDiscovery": true,
                "targets": ["pdf", "html", "bundle"]
            }
        }))
    }

    fn did_open(&mut self, params: OpenParams) -> Result<Value, RpcError> {
        let path = checked_relative_path(&params.path)?;
        let id = file_id(&path)?;
        self.documents.insert(
            path.clone(),
            OpenSource::new(path.clone(), id, params.text, params.version),
        );
        let config = self.project.configure_document(params.config);
        self.configs.insert(path, config);
        self.worlds.remove(&params.path);
        Ok(Value::Null)
    }

    fn did_change(&mut self, params: ChangeParams) -> Result<Value, RpcError> {
        let path = checked_relative_path(&params.path)?;
        let document = self
            .documents
            .get_mut(&path)
            .ok_or_else(|| invalid_params(format!("document is not open: {path}")))?;
        if params.version != document.version + 1 {
            return Err(RpcError {
                code: -32003,
                message: format!(
                    "revision mismatch for {path}: expected {}, received {}",
                    document.version + 1,
                    params.version
                ),
                data: Some(json!({"path": path, "expectedVersion": document.version + 1})),
            });
        }

        let mut dirty_start = usize::MAX;
        let mut dirty_end = 0;
        for change in params.changes {
            let dirty = document.edit(change).map_err(invalid_params)?;
            dirty_start = dirty_start.min(dirty.start);
            dirty_end = dirty_end.max(dirty.end);
        }
        document.version = params.version;
        if dirty_start == usize::MAX {
            dirty_start = 0;
        }
        serde_json::to_value(EditResult {
            version: params.version,
            dirty_start_line: dirty_start,
            dirty_end_line: dirty_end,
        })
        .map_err(internal_error)
    }

    fn full_sync(&mut self, params: FullSyncParams) -> Result<Value, RpcError> {
        let path = checked_relative_path(&params.path)?;
        let document = self
            .documents
            .get_mut(&path)
            .ok_or_else(|| invalid_params(format!("document is not open: {path}")))?;
        document.full_sync(params.text, params.version);
        Ok(Value::Null)
    }

    fn did_close(&mut self, params: PathParams) -> Result<Value, RpcError> {
        let path = checked_relative_path(&params.path)?;
        self.documents.remove(&path);
        self.configs.remove(&path);
        self.worlds.remove(&path);
        Ok(Value::Null)
    }

    fn equations(&self, params: VersionedPathParams) -> Result<Value, RpcError> {
        let path = checked_relative_path(&params.path)?;
        let document = self
            .documents
            .get(&path)
            .ok_or_else(|| invalid_params(format!("document is not open: {path}")))?;
        if params.version != document.version {
            return Err(RpcError {
                code: -32003,
                message: format!(
                    "revision mismatch for {path}: current {}, requested {}",
                    document.version, params.version
                ),
                data: Some(json!({"path": path, "expectedVersion": document.version})),
            });
        }
        Ok(json!({
            "documentVersion": document.version,
            "equations": document.equations()
        }))
    }

    fn status(&self) -> Value {
        json!({
            "protocolVersion": PROTOCOL_VERSION,
            "serverVersion": env!("CARGO_PKG_VERSION"),
            "typstVersion": typst_version(),
            "target": server_target(),
            "root": self.root,
            "initialized": self.initialized,
            "documents": self.documents.iter().map(|(path, document)| json!({
                "path": document.path,
                "version": document.version,
                "characters": document.char_len(),
                "target": self.configs.get(path).map(|config| config.target)
            })).collect::<Vec<_>>()
        })
    }

    fn render_math(&mut self, params: RenderMathParams) -> Result<Value, RpcError> {
        let path = checked_relative_path(&params.path)?;
        let document = self
            .documents
            .get(&path)
            .ok_or_else(|| invalid_params(format!("document is not open: {path}")))?;
        if params.version != document.version {
            return Err(RpcError {
                code: -32003,
                message: format!(
                    "revision mismatch for {path}: current {}, requested {}",
                    document.version, params.version
                ),
                data: Some(json!({"path": path, "expectedVersion": document.version})),
            });
        }
        let config = self.configs.get(&path).cloned().unwrap_or_default();
        self.prepare_world(&path)?;
        let world = self.worlds.get(&path).expect("prepared world exists");
        let document = self
            .documents
            .get(&path)
            .expect("validated open document exists");
        serde_json::to_value(render_math(world, document, &config, &params)).map_err(internal_error)
    }
}

fn parse<T: for<'de> Deserialize<'de>>(value: Value) -> Result<T, RpcError> {
    serde_json::from_value(value).map_err(|error| invalid_params(error.to_string()))
}

fn checked_relative_path(path: &str) -> Result<String, RpcError> {
    if path.is_empty() || path.contains('\\') {
        return Err(invalid_params(
            "document path must be a non-empty virtual path",
        ));
    }
    let candidate = Path::new(path);
    if candidate.is_absolute()
        || candidate.components().any(|component| {
            matches!(
                component,
                Component::ParentDir | Component::RootDir | Component::Prefix(_)
            )
        })
    {
        return Err(invalid_params("document path escapes the project root"));
    }
    Ok(path.into())
}

fn file_id(path: &str) -> Result<FileId, RpcError> {
    let virtual_path = VirtualPath::new(path)
        .map_err(|error| invalid_params(format!("invalid virtual path: {error}")))?;
    Ok(RootedPath::new(VirtualRoot::Project, virtual_path).intern())
}

fn invalid_request(message: impl Into<String>) -> RpcError {
    RpcError {
        code: -32600,
        message: message.into(),
        data: None,
    }
}

fn invalid_params(message: impl Into<String>) -> RpcError {
    RpcError {
        code: -32602,
        message: message.into(),
        data: None,
    }
}

fn internal_error(error: impl std::fmt::Display) -> RpcError {
    RpcError {
        code: -32603,
        message: error.to_string(),
        data: None,
    }
}

#[cfg(test)]
mod tests {
    use serde_json::json;
    use tempfile::tempdir;

    use super::*;

    fn test_server(root: PathBuf) -> Server {
        let project = WeibianProject::for_test(root.clone()).unwrap();
        Server::new(root, project).unwrap()
    }

    fn initialized() -> (tempfile::TempDir, Server) {
        let directory = tempdir().unwrap();
        let mut server = test_server(directory.path().into());
        server
            .handle(
                "initialize",
                json!({"protocolVersion": PROTOCOL_VERSION, "root": directory.path()}),
            )
            .unwrap();
        (directory, server)
    }

    #[test]
    fn handshake_rejects_mismatch() {
        let directory = tempdir().unwrap();
        let mut server = test_server(directory.path().into());
        let error = server
            .handle("initialize", json!({"protocolVersion": 99}))
            .unwrap_err();
        assert_eq!(error.code, -32001);
    }

    #[test]
    fn open_edit_resync_and_close() {
        let (_directory, mut server) = initialized();
        server
            .handle(
                "textDocument/didOpen",
                json!({"path":"main.typ","text":"A世界🙂","version":4}),
            )
            .unwrap();
        let edited = server
            .handle(
                "textDocument/didChange",
                json!({"path":"main.typ","version":5,"changes":[{"start":1,"end":3,"text":"中"}]}),
            )
            .unwrap();
        assert_eq!(edited["version"], 5);
        assert_eq!(server.documents["main.typ"].text(), "A中🙂");

        let mismatch = server
            .handle(
                "textDocument/didChange",
                json!({"path":"main.typ","version":8,"changes":[]}),
            )
            .unwrap_err();
        assert_eq!(mismatch.code, -32003);

        server
            .handle(
                "tfp/fullSync",
                json!({"path":"main.typ","text":"= Full","version":8}),
            )
            .unwrap();
        assert_eq!(server.document_version("main.typ"), Some(8));
        let equations = server
            .handle("tfp/equations", json!({"path":"main.typ","version":8}))
            .unwrap();
        assert_eq!(equations["documentVersion"], 8);
        assert_eq!(equations["equations"].as_array().unwrap().len(), 0);
        server
            .handle("textDocument/didClose", json!({"path":"main.typ"}))
            .unwrap();
        assert_eq!(server.document_version("main.typ"), None);
    }

    #[test]
    fn returns_parser_confirmed_equations_for_exact_version() {
        let (_directory, mut server) = initialized();
        server
            .handle(
                "textDocument/didOpen",
                json!({"path":"main.typ","text":"中 $x$ and $ y $","version":4}),
            )
            .unwrap();
        let result = server
            .handle("tfp/equations", json!({"path":"main.typ","version":4}))
            .unwrap();
        assert_eq!(result["equations"][0]["start"], 2);
        assert_eq!(result["equations"][0]["end"], 5);
        assert_eq!(result["equations"][0]["block"], false);
        assert_eq!(result["equations"][1]["block"], true);

        let mismatch = server
            .handle("tfp/equations", json!({"path":"main.typ","version":3}))
            .unwrap_err();
        assert_eq!(mismatch.code, -32003);
    }

    #[test]
    fn rejects_paths_outside_root() {
        let (_directory, mut server) = initialized();
        let error = server
            .handle(
                "textDocument/didOpen",
                json!({"path":"../secret.typ","text":"","version":0}),
            )
            .unwrap_err();
        assert_eq!(error.code, -32602);
    }
}
