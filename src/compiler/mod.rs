use std::collections::BTreeSet;

use crate::args::CompilerBackendKind;
use crate::config::BuildConfig;
use crate::error::StrResult;

#[cfg(feature = "host-compiler")]
pub mod host;
#[cfg(feature = "library-compiler")]
pub mod library;
#[cfg(feature = "library-compiler")]
pub(crate) mod world;

pub trait CompilerBackend {
    fn compile_bundle(build_config: &BuildConfig, entrypoint: &str) -> StrResult<()>;
}

pub fn compile_bundle(build_config: &BuildConfig, entrypoint: &str) -> StrResult<()> {
    match build_config.compiler_backend {
        CompilerBackendKind::Library => compile_with_library(build_config, entrypoint),
        CompilerBackendKind::Host => compile_with_host(build_config, entrypoint),
    }
}

#[cfg(feature = "library-compiler")]
fn compile_with_library(build_config: &BuildConfig, entrypoint: &str) -> StrResult<()> {
    library::LibraryCompiler::compile_bundle(build_config, entrypoint)
}

#[cfg(not(feature = "library-compiler"))]
fn compile_with_library(_build_config: &BuildConfig, _entrypoint: &str) -> StrResult<()> {
    Err(ecow::eco_format!(
        "compiler backend `library` was requested, but this binary was built without the `library-compiler` feature"
    ))
}

#[cfg(feature = "host-compiler")]
fn compile_with_host(build_config: &BuildConfig, entrypoint: &str) -> StrResult<()> {
    host::HostCompiler::compile_bundle(build_config, entrypoint)
}

#[cfg(not(feature = "host-compiler"))]
fn compile_with_host(_build_config: &BuildConfig, _entrypoint: &str) -> StrResult<()> {
    Err(ecow::eco_format!(
        "compiler backend `host` was requested, but this binary was built without the `host-compiler` feature"
    ))
}

pub(crate) fn forced_feature_names(env_features: Option<&str>) -> BTreeSet<String> {
    let mut features = BTreeSet::new();
    if let Some(env_features) = env_features {
        for feature in env_features.split(',') {
            let feature = feature.trim();
            if !feature.is_empty() {
                features.insert(feature.to_string());
            }
        }
    }
    features.insert("bundle".to_string());
    features.insert("html".to_string());
    features
}

#[cfg(feature = "host-compiler")]
pub(crate) fn forced_features(env_features: Option<&str>) -> String {
    forced_feature_names(env_features)
        .into_iter()
        .collect::<Vec<_>>()
        .join(",")
}

pub(crate) fn site_inputs(build_config: &BuildConfig) -> Vec<(String, String)> {
    vec![
        (
            "wb-domain".to_string(),
            build_config
                .site
                .domain
                .as_deref()
                .unwrap_or("")
                .to_string(),
        ),
        (
            "wb-root-dir".to_string(),
            build_config.site.root_dir.clone(),
        ),
        (
            "wb-trailing-slash".to_string(),
            if build_config.site.trailing_slash {
                "true".to_string()
            } else {
                "false".to_string()
            },
        ),
    ]
}
