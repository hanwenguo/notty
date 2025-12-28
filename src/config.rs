use std::path::{Path, PathBuf};

use ecow::eco_format;
use figment::Figment;
use figment::providers::{Format, Toml};
use serde::Deserialize;

use crate::args::{CompileArgs, ProcessArgs, WorldArgs};
use crate::error::StrResult;

const DEFAULT_CONFIG_PATH: &str = ".notty/config.toml";

#[derive(Debug, Default, Deserialize)]
pub struct NottyConfig {
    #[serde(default)]
    pub directories: DirectoriesConfig,

    #[serde(default)]
    pub site: SiteConfig,
}

#[derive(Debug, Default, Deserialize)]
pub struct DirectoriesConfig {
    pub input_dir: Option<PathBuf>,
    pub output_dir: Option<PathBuf>,
    pub public_dir: Option<PathBuf>,
    pub cache_dir: Option<PathBuf>,
}

#[derive(Debug, Default, Deserialize)]
pub struct SiteConfig {
    pub domain: Option<String>,
    pub root_dir: Option<String>,
    pub trailing_slash: Option<bool>,
}

#[derive(Debug, Clone)]
pub struct SiteSettings {
    #[allow(dead_code)]
    pub domain: Option<String>,
    pub root_dir: String,
    pub trailing_slash: bool,
}

/// A preprocessed `CompileCommand` with config defaults applied.
#[derive(Debug, Clone)]
pub struct BuildConfig {
    pub input_directory: PathBuf,
    pub html_cache_directory: PathBuf,
    pub public_directory: PathBuf,
    pub output_directory: PathBuf,
    pub site: SiteSettings,
    pub world: WorldArgs,
    pub process: ProcessArgs,
}

pub fn load_config(config_path: Option<&Path>) -> StrResult<NottyConfig> {
    let (path, is_default) = match config_path {
        Some(path) => (path.to_path_buf(), false),
        None => (PathBuf::from(DEFAULT_CONFIG_PATH), true),
    };

    if !path.exists() {
        if is_default {
            return Ok(NottyConfig::default());
        }
        return Err(eco_format!("config file {} does not exist", path.display()));
    }

    Figment::new()
        .merge(Toml::file(&path))
        .extract::<NottyConfig>()
        .map_err(|err| eco_format!("failed to load config {}: {err}", path.display()))
}

impl BuildConfig {
    pub fn from(args: &CompileArgs, config: &NottyConfig) -> StrResult<Self> {
        let input_directory = resolve_dir(
            args.input.as_ref(),
            config.directories.input_dir.as_ref(),
            "typ",
        );
        let html_cache_directory = resolve_dir(
            args.html_cache.as_ref(),
            config.directories.cache_dir.as_ref(),
            ".notty/cache",
        );
        let public_directory = resolve_dir(
            args.public.as_ref(),
            config.directories.public_dir.as_ref(),
            "public",
        );
        let output_directory = resolve_dir(
            args.output.as_ref(),
            config.directories.output_dir.as_ref(),
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
            html_cache_directory,
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
