//! TFP protocol version 3 server implementation.
//!
//! This stays wire-compatible with the reference server in tfp.el so `wb`
//! can serve as the client's long-lived formula preview process.

#[cfg(feature = "library-compiler")]
mod project;
#[cfg(feature = "library-compiler")]
mod protocol;
#[cfg(feature = "library-compiler")]
mod render;
#[cfg(feature = "library-compiler")]
mod server;
#[cfg(feature = "library-compiler")]
mod source;
#[cfg(feature = "library-compiler")]
mod world;

use std::path::Path;

#[cfg(not(feature = "library-compiler"))]
use ecow::eco_format;

use crate::error::StrResult;

/// Runs the TFP protocol server for one project root.
#[cfg(feature = "library-compiler")]
pub fn run(root: &Path, config_path: Option<&Path>) -> StrResult<()> {
    let project = project::WeibianProject::discover(root, config_path)?;
    run_server(root, project).map_err(Into::into)
}

#[cfg(not(feature = "library-compiler"))]
pub fn run(_root: &Path, _config_path: Option<&Path>) -> StrResult<()> {
    Err(eco_format!(
        "TFP server mode requires a binary built with the `library-compiler` feature"
    ))
}

#[cfg(feature = "library-compiler")]
fn run_server(root: &Path, project: project::WeibianProject) -> Result<(), String> {
    use std::collections::VecDeque;
    use std::io::{self, BufReader};
    use std::sync::mpsc;

    let server = server::Server::new(root.to_owned(), project)?;
    let (sender, receiver) = mpsc::channel();
    let worker = std::thread::Builder::new()
        .name("tfp-compiler".into())
        .spawn(move || worker_loop(server, receiver, VecDeque::new()))
        .map_err(|error| format!("cannot start compiler worker: {error}"))?;
    let stdin = io::stdin();
    let mut reader = BufReader::new(stdin.lock());

    while let Some(message) =
        protocol::read_message(&mut reader).map_err(|error| error.to_string())?
    {
        let is_exit = message.method.as_deref() == Some("exit");
        sender
            .send(message)
            .map_err(|_| "compiler worker stopped unexpectedly".to_string())?;
        if is_exit {
            break;
        }
    }
    drop(sender);
    worker
        .join()
        .map_err(|_| "compiler worker panicked".to_string())?
}

#[cfg(feature = "library-compiler")]
fn worker_loop(
    mut server: server::Server,
    receiver: std::sync::mpsc::Receiver<protocol::Incoming>,
    mut queue: std::collections::VecDeque<protocol::Incoming>,
) -> Result<(), String> {
    use std::io::{self, BufWriter};

    let stdout = io::stdout();
    let mut writer = BufWriter::new(stdout.lock());

    loop {
        if queue.is_empty() {
            match receiver.recv() {
                Ok(message) => queue.push_back(message),
                Err(_) => break,
            }
        }
        while let Ok(message) = receiver.try_recv() {
            queue.push_back(message);
        }

        let Some(message) = queue.pop_front() else {
            continue;
        };
        if superseded_render(&message, &queue) {
            if let Some(id) = &message.id {
                protocol::write_error(
                    &mut writer,
                    id,
                    protocol::RpcError {
                        code: -32800,
                        message: "render request superseded by a newer document revision".into(),
                        data: None,
                    },
                )
                .map_err(|error| error.to_string())?;
            }
            continue;
        }
        if handle_message(&mut server, message, &mut writer)? {
            break;
        }
    }
    Ok(())
}

#[cfg(feature = "library-compiler")]
fn superseded_render(
    message: &protocol::Incoming,
    queue: &std::collections::VecDeque<protocol::Incoming>,
) -> bool {
    if message.method.as_deref() != Some("tfp/renderMath") {
        return false;
    }
    let path = message.params.get("path").and_then(|value| value.as_str());
    let version = message
        .params
        .get("version")
        .and_then(|value| value.as_u64());
    queue.iter().any(|queued| {
        queued.method.as_deref() == Some("tfp/renderMath")
            && queued.params.get("path").and_then(|value| value.as_str()) == path
            && queued
                .params
                .get("version")
                .and_then(|value| value.as_u64())
                >= version
    })
}

#[cfg(feature = "library-compiler")]
fn handle_message<W: std::io::Write>(
    server: &mut server::Server,
    message: protocol::Incoming,
    writer: &mut W,
) -> Result<bool, String> {
    use std::panic::{AssertUnwindSafe, catch_unwind};

    if message.jsonrpc != "2.0" {
        if let Some(id) = &message.id {
            protocol::write_error(
                writer,
                id,
                protocol::RpcError {
                    code: -32600,
                    message: "unsupported JSON-RPC version".into(),
                    data: None,
                },
            )
            .map_err(|error| error.to_string())?;
        }
        return Ok(false);
    }
    let Some(method) = message.method else {
        return Ok(false);
    };
    let is_exit = method == "exit";
    let outcome = catch_unwind(AssertUnwindSafe(|| server.handle(&method, message.params)))
        .unwrap_or_else(|_| {
            Err(protocol::RpcError {
                code: -32603,
                message: "server request panicked".into(),
                data: None,
            })
        });

    if let Some(id) = &message.id {
        match outcome {
            Ok(result) => protocol::write_success(writer, id, result),
            Err(error) => protocol::write_error(writer, id, error),
        }
        .map_err(|error| error.to_string())?;
    } else if let Err(error) = outcome {
        eprintln!(
            "wb tfp-server: notification {method} failed: {}",
            error.message
        );
    }
    Ok(is_exit || (server.should_exit() && method == "exit"))
}

#[cfg(all(test, feature = "library-compiler"))]
mod tests {
    use std::collections::VecDeque;

    use serde_json::json;

    use super::*;

    fn incoming(id: u64, version: u64, path: &str) -> protocol::Incoming {
        protocol::Incoming {
            jsonrpc: "2.0".into(),
            id: Some(json!(id)),
            method: Some("tfp/renderMath".into()),
            params: json!({"path": path, "version": version}),
        }
    }

    #[test]
    fn detects_only_newer_render_for_same_document() {
        let current = incoming(1, 4, "main.typ");
        let queue = VecDeque::from([incoming(2, 9, "other.typ"), incoming(3, 5, "main.typ")]);
        assert!(superseded_render(&current, &queue));
        assert!(!superseded_render(&queue[0], &VecDeque::new()));
    }
}
