use std::collections::HashMap;
use std::io;
use std::path::PathBuf;
use std::sync::Arc;

use typst::World;
use typst::diag::{FileError, FileResult, PackageError, PackageResult};
use typst::foundations::{Bytes, Datetime, Dict, Duration, IntoValue};
use typst::syntax::{FileId, Source, VirtualRoot};
use typst::text::{Font, FontBook, TextElem};
use typst::visualize::Color;
use typst::{Feature, Features, Library, LibraryExt};
use typst_kit::datetime::Time;
use typst_kit::downloader::{Downloader, SystemDownloader};
use typst_kit::files::{FileLoader, FileStore, FsRoot};
use typst_kit::fonts::{self, FontStore};
use typst_kit::packages::{FsPackages, SystemPackages, UniversePackages};
use typst_utils::{LazyHash, hash128};

use crate::tfp_server::protocol::PROTOCOL_VERSION;

pub fn default_text_color() -> Color {
    Color::from_u8(0x2B, 0xE4, 0xB8, 0xFF)
}

#[derive(
    Clone, Copy, Debug, Default, Eq, Hash, PartialEq, serde::Deserialize, serde::Serialize,
)]
#[serde(rename_all = "lowercase")]
pub enum PreviewTarget {
    #[default]
    #[serde(alias = "paged")]
    Pdf,
    Html,
    Bundle,
}

#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq, serde::Deserialize, serde::Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum PreviewFeature {
    Html,
    Bundle,
    A11yExtras,
}

impl PreviewFeature {
    fn into_typst(self) -> Feature {
        match self {
            Self::Html => Feature::Html,
            Self::Bundle => Feature::Bundle,
            Self::A11yExtras => Feature::A11yExtras,
        }
    }
}

#[derive(Clone, Debug, Default, serde::Deserialize, serde::Serialize)]
#[serde(default, rename_all = "camelCase")]
pub struct DocumentConfig {
    pub target: PreviewTarget,
    pub font_paths: Vec<PathBuf>,
    pub package_paths: Vec<PathBuf>,
    pub package_cache_path: Option<PathBuf>,
    pub inputs: HashMap<String, String>,
    pub ignore_system_fonts: bool,
    pub ignore_embedded_fonts: bool,
    pub creation_timestamp: Option<i64>,
    pub features: Vec<PreviewFeature>,
    pub offline: bool,
}

#[derive(Debug)]
struct OfflineDownloader;

impl Downloader for OfflineDownloader {
    fn stream(
        &self,
        _key: &dyn std::any::Any,
        _url: &str,
    ) -> io::Result<(Option<usize>, Box<dyn io::Read>)> {
        Err(io::Error::new(io::ErrorKind::NotFound, "offline mode"))
    }
}

#[derive(Debug)]
enum PreviewDownloader {
    Offline(OfflineDownloader),
    Online(SystemDownloader),
}

impl Downloader for PreviewDownloader {
    fn stream(
        &self,
        key: &dyn std::any::Any,
        url: &str,
    ) -> io::Result<(Option<usize>, Box<dyn io::Read>)> {
        match self {
            Self::Offline(downloader) => downloader.stream(key, url),
            Self::Online(downloader) => downloader.stream(key, url),
        }
    }
}

#[derive(Debug)]
struct ProjectFiles {
    project: FsRoot,
    extra_packages: Vec<FsPackages>,
    packages: SystemPackages,
}

impl ProjectFiles {
    fn root(&self, id: FileId) -> PackageResult<FsRoot> {
        match id.root() {
            VirtualRoot::Project => Ok(self.project.clone()),
            VirtualRoot::Package(spec) => {
                for packages in &self.extra_packages {
                    if let Some(root) = packages.obtain(spec) {
                        return Ok(root);
                    }
                }
                self.packages.obtain(spec)
            }
        }
    }
}

impl FileLoader for ProjectFiles {
    fn load(&self, id: FileId) -> FileResult<Bytes> {
        self.root(id)
            .map_err(|error| match error {
                PackageError::NotFound(spec) => {
                    FileError::NotFound(PathBuf::from(spec.to_string()))
                }
                other => FileError::Package(other),
            })?
            .load(id.vpath())
    }
}

pub struct ProjectWorld {
    main: FileId,
    library: LazyHash<Library>,
    fonts: Arc<FontStore>,
    overlays: HashMap<FileId, Source>,
    preview_source: Option<Source>,
    files: FileStore<ProjectFiles>,
    time: Time,
}

impl ProjectWorld {
    pub fn load_fonts(config: &DocumentConfig) -> Arc<FontStore> {
        let mut font_store = FontStore::new();
        if !config.ignore_system_fonts {
            font_store.extend(fonts::system());
        }
        if !config.ignore_embedded_fonts {
            font_store.extend(fonts::embedded());
        }
        for path in &config.font_paths {
            font_store.extend(fonts::scan(path));
        }
        Arc::new(font_store)
    }

    #[cfg(test)]
    pub fn new(
        root: PathBuf,
        main: FileId,
        overlays: HashMap<FileId, Source>,
        config: &DocumentConfig,
    ) -> Result<Self, String> {
        Self::with_fonts(root, main, overlays, config, Self::load_fonts(config))
    }

    pub fn with_fonts(
        root: PathBuf,
        main: FileId,
        mut overlays: HashMap<FileId, Source>,
        config: &DocumentConfig,
        fonts: Arc<FontStore>,
    ) -> Result<Self, String> {
        let root = root
            .canonicalize()
            .map_err(|error| format!("cannot canonicalize project root: {error}"))?;

        let downloader = if config.offline {
            PreviewDownloader::Offline(OfflineDownloader)
        } else {
            PreviewDownloader::Online(SystemDownloader::new(format!(
                "weibian/{} tfp-protocol/{} typst/{}",
                env!("CARGO_PKG_VERSION"),
                PROTOCOL_VERSION,
                typst::utils::version().raw()
            )))
        };
        let packages = SystemPackages::from_parts(
            FsPackages::system_data(),
            config
                .package_cache_path
                .clone()
                .map(FsPackages::new)
                .or_else(FsPackages::system_cache),
            UniversePackages::new(downloader),
        );
        let files = ProjectFiles {
            project: FsRoot::new(root.clone()),
            extra_packages: config
                .package_paths
                .iter()
                .cloned()
                .map(FsPackages::new)
                .collect(),
            packages,
        };

        let inputs: Dict = config
            .inputs
            .iter()
            .map(|(key, value)| (key.as_str().into(), value.as_str().into_value()))
            .collect();
        let features: Features = config
            .features
            .iter()
            .copied()
            .map(PreviewFeature::into_typst)
            .chain(match config.target {
                PreviewTarget::Pdf => vec![],
                PreviewTarget::Html => vec![Feature::Html],
                PreviewTarget::Bundle => vec![Feature::Bundle, Feature::Html],
            })
            .collect();
        let mut library = Library::builder()
            .with_inputs(inputs)
            .with_features(features)
            .build();
        // Deliberately unusual, so the SVG adapter can map Typst's inherited
        // default text fill to CSS currentColor while preserving authored fills.
        library
            .styles
            .set(TextElem::fill, default_text_color().into());

        let (main, preview_source) = preview_main(main, config.target)?;
        if let Some(source) = &preview_source {
            overlays.insert(source.id(), source.clone());
        }
        let time = match config.creation_timestamp {
            Some(timestamp) => Time::fixed_timestamp(timestamp)
                .map_err(|error| format!("invalid creation timestamp: {error}"))?,
            None => Time::system(),
        };

        Ok(Self {
            main,
            library: LazyHash::new(library),
            fonts,
            overlays,
            preview_source,
            files: FileStore::new(files),
            time,
        })
    }

    pub fn prepare_compile(&mut self, mut overlays: HashMap<FileId, Source>) {
        if let Some(source) = &self.preview_source {
            overlays.insert(source.id(), source.clone());
        }
        self.overlays = overlays;
        // Reload disk dependencies, retaining stale Source objects so Typst's
        // parser and compiler can reuse them incrementally.
        self.files.reset();
        self.time.reset();
    }
}

fn preview_main(main: FileId, target: PreviewTarget) -> Result<(FileId, Option<Source>), String> {
    if target == PreviewTarget::Pdf {
        return Ok((main, None));
    }

    let wrapper_path = format!(".tfp/preview-{:032x}.typ", hash128(&main));
    let wrapper_id = typst::syntax::RootedPath::new(
        VirtualRoot::Project,
        typst::syntax::VirtualPath::new(&wrapper_path)
            .map_err(|error| format!("cannot create preview wrapper path: {error}"))?,
    )
    .intern();
    let path = format!("/{}", main.vpath().get_without_slash());
    let quoted = serde_json::to_string(&path)
        .map_err(|error| format!("cannot quote preview main path: {error}"))?;
    let text = format!(
        "#show math.equation: it => if it.block == true {{ block(html.frame(it)) }} else {{ html.frame(it) }}\n#include {quoted}\n"
    );
    Ok((wrapper_id, Some(Source::new(wrapper_id, text))))
}

impl World for ProjectWorld {
    fn library(&self) -> &LazyHash<Library> {
        &self.library
    }

    fn book(&self) -> &LazyHash<FontBook> {
        self.fonts.book()
    }

    fn main(&self) -> FileId {
        self.main
    }

    fn source(&self, id: FileId) -> FileResult<Source> {
        self.overlays
            .get(&id)
            .cloned()
            .map(Ok)
            .unwrap_or_else(|| self.files.source(id))
    }

    fn file(&self, id: FileId) -> FileResult<Bytes> {
        self.overlays
            .get(&id)
            .cloned()
            .map(|source| Ok(Bytes::from_string(source)))
            .unwrap_or_else(|| self.files.file(id))
    }

    fn font(&self, index: usize) -> Option<Font> {
        self.fonts.font(index)
    }

    fn today(&self, offset: Option<Duration>) -> Option<Datetime> {
        self.time.today(offset)
    }
}

#[cfg(test)]
mod tests {
    use typst::syntax::{RootedPath, VirtualPath};
    use typst_layout::PagedDocument;

    use super::*;

    fn id(path: &str) -> FileId {
        RootedPath::new(
            VirtualRoot::Project,
            VirtualPath::new(path).expect("valid test path"),
        )
        .intern()
    }

    #[test]
    fn compiles_with_unsaved_import_overlay() {
        let directory = tempfile::tempdir().unwrap();
        std::fs::write(directory.path().join("part.typ"), "disk text").unwrap();

        let main = id("main.typ");
        let part = id("part.typ");
        let overlays = HashMap::from([
            (main, Source::new(main, "#include \"part.typ\"".into())),
            (part, Source::new(part, "unsaved text".into())),
        ]);
        let config = DocumentConfig {
            ignore_system_fonts: true,
            offline: true,
            ..DocumentConfig::default()
        };
        let world = ProjectWorld::new(directory.path().into(), main, overlays, &config).unwrap();
        let result = typst::compile::<PagedDocument>(&world);
        assert!(result.output.is_ok(), "{:?}", result.output.err());
        assert_eq!(world.source(part).unwrap().text(), "unsaved text");
    }

    #[test]
    fn reloads_disk_sources_while_retaining_the_world() {
        let directory = tempfile::tempdir().unwrap();
        std::fs::write(directory.path().join("part.typ"), "first").unwrap();
        let main = id("main.typ");
        let part = id("part.typ");
        let overlays = HashMap::from([(main, Source::new(main, "#include \"part.typ\"".into()))]);
        let config = DocumentConfig {
            ignore_system_fonts: true,
            offline: true,
            ..DocumentConfig::default()
        };
        let mut world =
            ProjectWorld::new(directory.path().into(), main, overlays.clone(), &config).unwrap();
        assert_eq!(world.source(part).unwrap().text(), "first");
        std::fs::write(directory.path().join("part.typ"), "second").unwrap();
        world.prepare_compile(overlays);
        assert_eq!(world.source(part).unwrap().text(), "second");
    }
}
