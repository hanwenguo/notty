use std::fmt;
use std::path::{Path, PathBuf};

use ecow::eco_format;
use figment::Figment;
use figment::providers::{Format, Toml};
use globset::{Glob, GlobSet, GlobSetBuilder};
use serde::de::{self, SeqAccess, Visitor};
use serde::{Deserialize, Deserializer};

use crate::args::{CompileArgs, ProcessArgs, WorldArgs};
use crate::error::StrResult;

const DEFAULT_CONFIG_PATH: &str = ".wb/config.toml";

#[derive(Debug, Default, Deserialize)]
pub struct WeibianConfig {
    #[serde(default, alias = "directories")]
    pub files: FilesConfig,

    #[serde(default)]
    pub site: SiteConfig,
}

#[derive(Debug, Default, Deserialize)]
pub struct FilesConfig {
    pub input_dir: Option<PathBuf>,
    pub output_dir: Option<PathBuf>,
    pub public_dir: Option<PathBuf>,
    #[serde(default, deserialize_with = "deserialize_glob_list")]
    pub include: Vec<String>,
    #[serde(default, deserialize_with = "deserialize_glob_list")]
    pub exclude: Vec<String>,
}

#[derive(Debug, Default, Deserialize)]
pub struct SiteConfig {
    pub domain: Option<String>,
    pub root_dir: Option<String>,
    pub trailing_slash: Option<bool>,
}
#[derive(Debug, Clone)]
pub struct SiteSettings {
    pub domain: Option<String>,
    pub root_dir: String,
    pub trailing_slash: bool,
}

#[derive(Debug, Clone)]
pub struct InputFilters {
    include: GlobSet,
    exclude: GlobSet,
    include_all: bool,
    has_exclude: bool,
}

impl InputFilters {
    pub fn new(include: &[String], exclude: &[String]) -> StrResult<Self> {
        let include_set = build_globset(include, "include")?;
        let exclude_set = build_globset(exclude, "exclude")?;
        Ok(Self {
            include: include_set,
            exclude: exclude_set,
            include_all: include.is_empty(),
            has_exclude: !exclude.is_empty(),
        })
    }

    pub fn allows(&self, relative_path: &Path) -> bool {
        if self.exclude.is_match(relative_path) {
            return false;
        }
        if self.include_all {
            return true;
        }
        self.include.is_match(relative_path)
    }

    pub fn has_filters(&self) -> bool {
        self.has_exclude || !self.include_all
    }
}

/// A preprocessed `CompileCommand` with config defaults applied.
#[derive(Debug, Clone)]
pub struct BuildConfig {
    pub input_directory: PathBuf,
    pub input_filters: InputFilters,
    pub public_directory: PathBuf,
    pub output_directory: PathBuf,
    pub site: SiteSettings,
    pub world: WorldArgs,
    pub process: ProcessArgs,
}

pub fn load_config(config_path: Option<&Path>) -> StrResult<WeibianConfig> {
    let (path, is_default) = match config_path {
        Some(path) => (path.to_path_buf(), false),
        None => (PathBuf::from(DEFAULT_CONFIG_PATH), true),
    };

    if !path.exists() {
        if is_default {
            return Ok(WeibianConfig::default());
        }
        return Err(eco_format!("config file {} does not exist", path.display()));
    }

    Figment::new()
        .merge(Toml::file(&path))
        .extract::<WeibianConfig>()
        .map_err(|err| eco_format!("failed to load config {}: {err}", path.display()))
}

impl BuildConfig {
    pub fn from(args: &CompileArgs, config: &WeibianConfig) -> StrResult<Self> {
        let input_directory =
            resolve_dir(args.input.as_ref(), config.files.input_dir.as_ref(), "typ");
        let input_filters = InputFilters::new(&config.files.include, &config.files.exclude)?;
        let public_directory = resolve_dir(
            args.public.as_ref(),
            config.files.public_dir.as_ref(),
            "public",
        );
        let output_directory = resolve_dir(
            args.output.as_ref(),
            config.files.output_dir.as_ref(),
            "dist",
        );

        let domain = args
            .site
            .domain
            .clone()
            .or_else(|| config.site.domain.clone());
        let root_dir = normalize_root_dir(
            args.site
                .root_dir
                .as_deref()
                .or(config.site.root_dir.as_deref()),
        );
        let trailing_slash = args
            .site
            .trailing_slash
            .unwrap_or(config.site.trailing_slash.unwrap_or(false));
        Ok(Self {
            input_directory,
            input_filters,
            public_directory,
            output_directory,
            site: SiteSettings {
                domain,
                root_dir,
                trailing_slash,
            },
            world: args.world.clone(),
            process: args.process.clone(),
        })
    }
}

fn build_globset(patterns: &[String], label: &str) -> StrResult<GlobSet> {
    let mut builder = GlobSetBuilder::new();
    for pattern in patterns {
        let glob = Glob::new(pattern)
            .map_err(|err| eco_format!("invalid {label} glob \"{pattern}\": {err}"))?;
        builder.add(glob);
    }
    builder
        .build()
        .map_err(|err| eco_format!("failed to build {label} glob set: {err}"))
}

fn deserialize_glob_list<'de, D>(deserializer: D) -> Result<Vec<String>, D::Error>
where
    D: Deserializer<'de>,
{
    struct GlobListVisitor;

    impl<'de> Visitor<'de> for GlobListVisitor {
        type Value = Vec<String>;

        fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
            formatter.write_str("a string or a list of strings")
        }

        fn visit_str<E>(self, value: &str) -> Result<Self::Value, E>
        where
            E: de::Error,
        {
            Ok(vec![value.to_string()])
        }

        fn visit_string<E>(self, value: String) -> Result<Self::Value, E>
        where
            E: de::Error,
        {
            Ok(vec![value])
        }

        fn visit_seq<A>(self, mut seq: A) -> Result<Self::Value, A::Error>
        where
            A: SeqAccess<'de>,
        {
            let mut values = Vec::new();
            while let Some(value) = seq.next_element::<String>()? {
                values.push(value);
            }
            Ok(values)
        }
    }

    deserializer.deserialize_any(GlobListVisitor)
}

fn resolve_dir(cli: Option<&PathBuf>, config: Option<&PathBuf>, default: &str) -> PathBuf {
    cli.cloned()
        .or_else(|| config.cloned())
        .unwrap_or_else(|| PathBuf::from(default))
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
