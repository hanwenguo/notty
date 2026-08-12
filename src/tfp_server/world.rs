use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;

use typst::foundations::{Dict, IntoValue};
use typst::syntax::{FileId, RootedPath, Source, VirtualPath, VirtualRoot};
use typst::text::TextElem;
use typst::visualize::Color;
use typst::{Feature, Features, Library, LibraryExt};
use typst_kit::fonts::FontStore;
use typst_utils::hash128;

use crate::compiler::world::{LibraryWorld, WorldOptions, discover_fonts};
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

pub(crate) fn load_fonts(config: &DocumentConfig) -> Arc<FontStore> {
    discover_fonts(
        &config.font_paths,
        config.ignore_system_fonts,
        config.ignore_embedded_fonts,
    )
}

pub(crate) fn create_project_world(
    root: PathBuf,
    main: FileId,
    mut sources: HashMap<FileId, Source>,
    config: &DocumentConfig,
    fonts: Arc<FontStore>,
) -> Result<LibraryWorld, String> {
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

    let main = add_preview_wrapper(main, config.target, &mut sources)?;
    LibraryWorld::new(WorldOptions {
        root,
        main,
        main_name: None,
        sources,
        library,
        fonts,
        extra_package_paths: config.package_paths.clone(),
        package_path: None,
        package_cache_path: config.package_cache_path.clone(),
        creation_timestamp: config.creation_timestamp,
        offline: config.offline,
        user_agent: format!(
            "weibian/{} tfp-protocol/{} typst/{}",
            env!("CARGO_PKG_VERSION"),
            PROTOCOL_VERSION,
            typst::utils::version().raw()
        ),
    })
}

pub(crate) fn prepare_project_world(
    world: &mut LibraryWorld,
    main: FileId,
    mut sources: HashMap<FileId, Source>,
    target: PreviewTarget,
) -> Result<(), String> {
    let main = add_preview_wrapper(main, target, &mut sources)?;
    world.replace_sources(main, sources);
    world.reset();
    Ok(())
}

fn add_preview_wrapper(
    main: FileId,
    target: PreviewTarget,
    sources: &mut HashMap<FileId, Source>,
) -> Result<FileId, String> {
    if target == PreviewTarget::Pdf {
        return Ok(main);
    }

    let wrapper_path = format!(".tfp/preview-{:032x}.typ", hash128(&main));
    let wrapper_id = RootedPath::new(
        VirtualRoot::Project,
        VirtualPath::new(&wrapper_path)
            .map_err(|error| format!("cannot create preview wrapper path: {error}"))?,
    )
    .intern();
    let path = format!("/{}", main.vpath().get_without_slash());
    let quoted = serde_json::to_string(&path)
        .map_err(|error| format!("cannot quote preview main path: {error}"))?;
    let text = format!(
        "#show math.equation: it => if it.block == true {{ block(html.frame(it)) }} else {{ html.frame(it) }}\n#include {quoted}\n"
    );
    sources.insert(wrapper_id, Source::new(wrapper_id, text));
    Ok(wrapper_id)
}
