use std::collections::{BTreeSet, HashMap};
use std::path::{Path, PathBuf};

use ecow::eco_format;
use typst::syntax::{FileId, RootedPath, Source, VirtualPath, VirtualRoot};

use crate::bundle::{TypstSource, collect_asset_files, collect_typst_sources, render_entrypoint};
use crate::config::{InputFilters, WeibianConfig, load_config};
use crate::error::StrResult;
use crate::tfp_server::world::{DocumentConfig, PreviewFeature, PreviewTarget};

/// Weibian-specific project data needed to compile formula previews through
/// the same synthetic bundle entrypoint as `wb compile`.
pub struct WeibianProject {
    root: PathBuf,
    filters: InputFilters,
    public_directory: PathBuf,
    site_inputs: HashMap<String, String>,
    entrypoint: FileId,
}

impl WeibianProject {
    pub fn discover(root: &Path, explicit_config: Option<&Path>) -> StrResult<Self> {
        let root = root.canonicalize().map_err(|error| {
            eco_format!(
                "cannot canonicalize TFP project root {}: {error}",
                root.display()
            )
        })?;

        if let Some(path) = explicit_config {
            let path = absolute_path(path)?;
            let config = load_config(Some(&path))?;
            return Self::from_config(&root, &path, config);
        }

        for directory in root.ancestors() {
            let path = directory.join("weibian.toml");
            if !path.is_file() {
                continue;
            }
            let config = load_config(Some(&path))?;
            if let Some(project) = Self::from_ancestor_config(&root, &path, config)? {
                return Ok(project);
            }
        }

        Err(eco_format!(
            "no weibian.toml found whose input directory matches TFP project root {}",
            root.display()
        ))
    }

    fn from_config(root: &Path, path: &Path, config: WeibianConfig) -> StrResult<Self> {
        let input_directory = configured_input_directory(path, &config)?;
        if input_directory != root {
            return Err(eco_format!(
                "TFP project root {} differs from input directory {} configured by {}",
                root.display(),
                input_directory.display(),
                path.display()
            ));
        }
        Self::build(root.to_owned(), config)
    }

    fn from_ancestor_config(
        root: &Path,
        path: &Path,
        config: WeibianConfig,
    ) -> StrResult<Option<Self>> {
        let input_directory = configured_input_directory(path, &config)?;
        if input_directory != root {
            return Ok(None);
        }
        Self::build(root.to_owned(), config).map(Some)
    }

    fn build(root: PathBuf, config: WeibianConfig) -> StrResult<Self> {
        let filters = InputFilters::new(&config.files.include, &config.files.exclude)?;
        let public = config
            .files
            .public_dir
            .unwrap_or_else(|| PathBuf::from("public"));
        let public_directory = if public.is_absolute() {
            public
        } else {
            root.join(public)
        };
        if public_directory.exists() {
            let public = public_directory.canonicalize().map_err(|error| {
                eco_format!(
                    "cannot canonicalize public directory {}: {error}",
                    public_directory.display()
                )
            })?;
            if !public.starts_with(&root) {
                return Err(eco_format!(
                    "public directory {} must be inside input directory {}",
                    public_directory.display(),
                    root.display()
                ));
            }
        }

        let site_inputs = HashMap::from([
            (
                "wb-domain".to_string(),
                config.site.domain.unwrap_or_default(),
            ),
            (
                "wb-root-dir".to_string(),
                normalize_root_dir(config.site.root_dir.as_deref()),
            ),
            (
                "wb-trailing-slash".to_string(),
                config.site.trailing_slash.unwrap_or(false).to_string(),
            ),
        ]);
        let entrypoint = RootedPath::new(
            VirtualRoot::Project,
            VirtualPath::new(".tfp/weibian-entrypoint.typ")
                .expect("synthetic TFP entrypoint path is valid"),
        )
        .intern();

        Ok(Self {
            root,
            filters,
            public_directory,
            site_inputs,
            entrypoint,
        })
    }

    #[cfg(test)]
    pub fn for_test(root: PathBuf) -> StrResult<Self> {
        Self::build(root, WeibianConfig::default())
    }

    pub fn configure_document(&self, mut config: DocumentConfig) -> DocumentConfig {
        // Weibian sources are bundle documents. The client defaults to PDF,
        // but compiling one note with paged semantics bypasses the synthetic
        // entrypoint and makes `document` calls invalid.
        config.target = PreviewTarget::Bundle;
        for feature in [PreviewFeature::Bundle, PreviewFeature::Html] {
            if !config.features.contains(&feature) {
                config.features.push(feature);
            }
        }
        for (key, value) in &self.site_inputs {
            config.inputs.entry(key.clone()).or_insert(value.clone());
        }
        config
    }

    pub fn entrypoint_source<'a>(
        &self,
        open_paths: impl IntoIterator<Item = &'a str>,
    ) -> StrResult<Source> {
        let mut sources = collect_typst_sources(&self.root, &self.filters)?;
        let mut known = sources
            .iter()
            .map(|source| source.root_relative.clone())
            .collect::<BTreeSet<_>>();

        // A newly created included buffer may not exist on disk yet. Include
        // it when its virtual path passes the same filters as saved sources.
        for path in open_paths {
            if !path.ends_with(".typ") || !self.filters.allows(Path::new(path)) {
                continue;
            }
            let root_relative = format!("/{path}");
            if known.insert(root_relative.clone()) {
                sources.push(TypstSource { root_relative });
            }
        }
        sources.sort_by(|left, right| left.root_relative.cmp(&right.root_relative));

        let assets = collect_asset_files(&self.root, &self.public_directory)?;
        Ok(Source::new(
            self.entrypoint,
            render_entrypoint(&sources, &assets),
        ))
    }
}

fn configured_input_directory(path: &Path, config: &WeibianConfig) -> StrResult<PathBuf> {
    let base = path.parent().unwrap_or_else(|| Path::new("."));
    let input = config
        .files
        .input_dir
        .clone()
        .unwrap_or_else(|| PathBuf::from("typ"));
    let input = if input.is_absolute() {
        input
    } else {
        base.join(input)
    };
    input.canonicalize().map_err(|error| {
        eco_format!(
            "cannot canonicalize configured input directory {}: {error}",
            input.display()
        )
    })
}

fn absolute_path(path: &Path) -> StrResult<PathBuf> {
    if path.is_absolute() {
        return Ok(path.to_owned());
    }
    std::env::current_dir()
        .map(|directory| directory.join(path))
        .map_err(|error| eco_format!("cannot resolve config path {}: {error}", path.display()))
}

fn normalize_root_dir(raw: Option<&str>) -> String {
    let mut root = raw.unwrap_or("/").trim().to_string();
    if root.is_empty() {
        root = "/".to_string();
    }
    if !root.starts_with('/') {
        root.insert(0, '/');
    }
    if !root.ends_with('/') {
        root.push('/');
    }
    root
}

#[cfg(test)]
mod tests {
    use std::fs;

    use tempfile::tempdir;

    use super::*;

    #[test]
    fn discovers_parent_config_and_builds_filtered_entrypoint() {
        let directory = tempdir().unwrap();
        let input = directory.path().join("typ");
        fs::create_dir_all(input.join("_template")).unwrap();
        fs::create_dir_all(input.join("public")).unwrap();
        fs::write(input.join("note.typ"), "#document(\"index.html\")[Note]").unwrap();
        fs::write(input.join("_template/helper.typ"), "#let helper = 1").unwrap();
        fs::write(input.join("public/site.css"), "body {}").unwrap();
        fs::write(
            directory.path().join("weibian.toml"),
            r#"
                [files]
                input_dir = "typ"
                public_dir = "public"
                exclude = ["_template/*.typ"]

                [site]
                domain = "https://example.com"
                root_dir = "notes"
                trailing_slash = true
            "#,
        )
        .unwrap();

        let project = WeibianProject::discover(&input, None).unwrap();
        let source = project
            .entrypoint_source(["new.typ", "_template/open.typ"])
            .unwrap();
        assert!(source.text().contains("#include \"/note.typ\""));
        assert!(source.text().contains("#include \"/new.typ\""));
        assert!(!source.text().contains("_template"));
        assert!(source.text().contains("#asset(\"site.css\""));

        let config = project.configure_document(DocumentConfig::default());
        assert_eq!(config.target, PreviewTarget::Bundle);
        assert_eq!(config.inputs["wb-domain"], "https://example.com");
        assert_eq!(config.inputs["wb-root-dir"], "/notes/");
        assert_eq!(config.inputs["wb-trailing-slash"], "true");
    }
}
