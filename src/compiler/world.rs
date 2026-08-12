use std::collections::HashMap;
use std::io;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use typst::World;
use typst::diag::{FileError, FileResult, PackageError, PackageResult};
use typst::foundations::{Bytes, Datetime, Duration};
use typst::syntax::{FileId, Source, VirtualRoot};
use typst::text::{Font, FontBook};
use typst::{Library, utils::LazyHash};
use typst_kit::datetime::Time;
use typst_kit::diagnostics::DiagnosticWorld;
use typst_kit::downloader::{Downloader, SystemDownloader};
use typst_kit::files::{FileLoader, FileStore, FsRoot};
use typst_kit::fonts::{self, FontStore};
use typst_kit::packages::{FsPackages, SystemPackages, UniversePackages};

pub(crate) struct WorldOptions {
    pub root: PathBuf,
    pub main: FileId,
    pub main_name: Option<String>,
    pub sources: HashMap<FileId, Source>,
    pub library: Library,
    pub fonts: Arc<FontStore>,
    pub extra_package_paths: Vec<PathBuf>,
    pub package_path: Option<PathBuf>,
    pub package_cache_path: Option<PathBuf>,
    pub creation_timestamp: Option<i64>,
    pub offline: bool,
    pub user_agent: String,
}

pub(crate) struct LibraryWorld {
    workdir: Option<PathBuf>,
    main: FileId,
    main_name: Option<String>,
    library: LazyHash<Library>,
    fonts: Arc<FontStore>,
    files: FileStore<WorldFiles>,
    time: Time,
}

impl LibraryWorld {
    pub fn new(options: WorldOptions) -> Result<Self, String> {
        let root = options
            .root
            .canonicalize()
            .map_err(|error| format!("cannot canonicalize project root: {error}"))?;
        let downloader = if options.offline {
            WorldDownloader::Offline(OfflineDownloader)
        } else {
            WorldDownloader::Online(SystemDownloader::new(options.user_agent))
        };
        let packages = SystemPackages::from_parts(
            options
                .package_path
                .map(FsPackages::new)
                .or_else(FsPackages::system_data),
            options
                .package_cache_path
                .map(FsPackages::new)
                .or_else(FsPackages::system_cache),
            UniversePackages::new(downloader),
        );
        let time = match options.creation_timestamp {
            Some(timestamp) => Time::fixed_timestamp(timestamp)
                .map_err(|error| format!("invalid creation timestamp: {error}"))?,
            None => Time::system(),
        };

        Ok(Self {
            workdir: std::env::current_dir().ok(),
            main: options.main,
            main_name: options.main_name,
            library: LazyHash::new(options.library),
            fonts: options.fonts,
            files: FileStore::new(WorldFiles {
                project: FsRoot::new(root),
                extra_packages: options
                    .extra_package_paths
                    .into_iter()
                    .map(FsPackages::new)
                    .collect(),
                packages,
                sources: options.sources,
            }),
            time,
        })
    }

    pub fn root(&self) -> &Path {
        self.files.loader().project.path()
    }

    pub fn workdir(&self) -> &Path {
        self.workdir.as_deref().unwrap_or(Path::new("."))
    }

    pub fn dependencies(&mut self) -> impl Iterator<Item = PathBuf> + '_ {
        let (loader, dependencies) = self.files.dependencies();
        dependencies.filter_map(|id| loader.resolve(id).ok())
    }

    pub fn reset(&mut self) {
        self.files.reset();
        self.time.reset();
    }

    pub fn replace_sources(&mut self, main: FileId, sources: HashMap<FileId, Source>) {
        self.main = main;
        self.files.loader_mut().sources = sources;
    }
}

impl World for LibraryWorld {
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
        self.files
            .loader()
            .sources
            .get(&id)
            .cloned()
            .map(Ok)
            .unwrap_or_else(|| self.files.source(id))
    }

    fn file(&self, id: FileId) -> FileResult<Bytes> {
        self.files
            .loader()
            .sources
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

impl DiagnosticWorld for LibraryWorld {
    fn name(&self, id: FileId) -> String {
        if id == self.main
            && let Some(name) = &self.main_name
        {
            return name.clone();
        }

        let vpath = id.vpath();
        match id.root() {
            VirtualRoot::Project => {
                let rooted = vpath.realize(self.root()).ok();
                rooted
                    .and_then(|path| pathdiff::diff_paths(path, self.workdir()))
                    .map(|path| path.to_string_lossy().into_owned())
                    .unwrap_or_else(|| vpath.get_without_slash().into())
            }
            VirtualRoot::Package(package) => format!("{package}{}", vpath.get_with_slash()),
        }
    }
}

#[derive(Debug)]
struct WorldFiles {
    project: FsRoot,
    extra_packages: Vec<FsPackages>,
    packages: SystemPackages,
    sources: HashMap<FileId, Source>,
}

impl WorldFiles {
    fn resolve(&self, id: FileId) -> FileResult<PathBuf> {
        self.root(id)?.resolve(id.vpath())
    }

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

impl FileLoader for WorldFiles {
    fn load(&self, id: FileId) -> FileResult<Bytes> {
        if let Some(source) = self.sources.get(&id) {
            return Ok(Bytes::from_string(source.clone()));
        }

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

pub(crate) fn discover_fonts(
    paths: &[PathBuf],
    ignore_system_fonts: bool,
    ignore_embedded_fonts: bool,
) -> Arc<FontStore> {
    let mut store = FontStore::new();
    if !ignore_system_fonts {
        store.extend(fonts::system());
    }
    if !ignore_embedded_fonts {
        store.extend(fonts::embedded());
    }
    for path in paths {
        store.extend(fonts::scan(path));
    }
    Arc::new(store)
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
enum WorldDownloader {
    Offline(OfflineDownloader),
    Online(SystemDownloader),
}

impl Downloader for WorldDownloader {
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

#[cfg(test)]
mod tests {
    use super::*;
    use typst::LibraryExt;
    use typst::syntax::{RootedPath, VirtualPath};

    fn id(path: &str) -> FileId {
        RootedPath::new(
            VirtualRoot::Project,
            VirtualPath::new(path).expect("valid test path"),
        )
        .intern()
    }

    fn world(root: PathBuf, main: FileId, sources: HashMap<FileId, Source>) -> LibraryWorld {
        LibraryWorld::new(WorldOptions {
            root,
            main,
            main_name: None,
            sources,
            library: Library::builder().build(),
            fonts: Arc::new(FontStore::new()),
            extra_package_paths: vec![],
            package_path: None,
            package_cache_path: None,
            creation_timestamp: None,
            offline: true,
            user_agent: "weibian-test".into(),
        })
        .unwrap()
    }

    #[test]
    fn overlays_unsaved_sources_and_reloads_disk_sources() {
        let directory = tempfile::tempdir().unwrap();
        std::fs::write(directory.path().join("part.typ"), "disk text").unwrap();
        let main = id("main.typ");
        let part = id("part.typ");
        let sources = HashMap::from([
            (main, Source::new(main, "#include \"part.typ\"".into())),
            (part, Source::new(part, "unsaved text".into())),
        ]);
        let mut world = world(directory.path().into(), main, sources);

        assert_eq!(world.source(part).unwrap().text(), "unsaved text");
        world.replace_sources(
            main,
            HashMap::from([(main, Source::new(main, "#include \"part.typ\"".into()))]),
        );
        world.reset();
        assert_eq!(world.source(part).unwrap().text(), "disk text");
    }
}
