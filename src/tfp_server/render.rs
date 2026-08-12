use std::collections::HashSet;
use std::time::Instant;

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use typst::World;
use typst::diag::{Severity, SourceDiagnostic, SourceResult, Warned};
use typst::foundations::{Content, Smart};
use typst::introspection::{Location, Tag};
use typst::layout::{Abs, Frame, FrameItem, GroupItem, Point, Rect, Sides, Size, Transform};
use typst::math::EquationElem;
use typst::{WorldExt, compile};
use typst_bundle::{Bundle, BundleDocument, BundleFile};
use typst_html::{HtmlDocument, HtmlElement, HtmlNode};
use typst_layout::{Page, PagedDocument};
use typst_svg::SvgOptions;
use typst_syntax::FileId;
use typst_utils::hash128;

use crate::compiler::world::LibraryWorld;
use crate::tfp_server::protocol::{PROTOCOL_VERSION, typst_version};
use crate::tfp_server::source::OpenSource;
use crate::tfp_server::world::{DocumentConfig, PreviewTarget, default_text_color};

#[derive(Clone, Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct EquationRequest {
    pub start: usize,
    pub end: usize,
    #[serde(default)]
    pub block: bool,
}

#[derive(Clone, Debug, Deserialize)]
#[serde(default, rename_all = "camelCase")]
pub struct RenderMathParams {
    pub path: String,
    pub version: u64,
    pub equations: Vec<EquationRequest>,
    pub padding_pt: f64,
    pub known_render_keys: HashSet<String>,
}

impl Default for RenderMathParams {
    fn default() -> Self {
        Self {
            path: String::new(),
            version: 0,
            equations: vec![],
            padding_pt: 1.0,
            known_render_keys: HashSet::new(),
        }
    }
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RenderMathResult {
    pub document_version: u64,
    pub source_fingerprint: String,
    pub diagnostics: Vec<Diagnostic>,
    pub equations: Vec<EquationResult>,
    pub timings: RenderTimings,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct EquationResult {
    pub start: usize,
    pub end: usize,
    pub block: bool,
    pub source_hash: String,
    pub render_key: Option<String>,
    pub status: EquationStatus,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub svg: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub width_pt: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub height_pt: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub baseline_pt: Option<f64>,
    pub occurrence_count: usize,
}

#[derive(Clone, Copy, Debug, Serialize, Eq, PartialEq)]
#[serde(rename_all = "kebab-case")]
pub enum EquationStatus {
    Ok,
    Hidden,
    NotLaidOut,
    Error,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Diagnostic {
    pub path: Option<String>,
    pub start: Option<usize>,
    pub end: Option<usize>,
    pub severity: &'static str,
    pub message: String,
    pub hints: Vec<String>,
    pub trace: Vec<String>,
}

#[derive(Clone, Debug, Default, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RenderTimings {
    pub compile_ms: f64,
    pub extraction_ms: f64,
    pub svg_ms: f64,
    pub total_ms: f64,
}

pub fn render_math(
    world: &LibraryWorld,
    document: &OpenSource,
    config: &DocumentConfig,
    params: &RenderMathParams,
) -> RenderMathResult {
    let total_started = Instant::now();
    let source_fingerprint = digest(document.text().as_bytes());
    let compile_started = Instant::now();
    let mut raw_diagnostics = vec![];
    let output = match config.target {
        PreviewTarget::Pdf => collect_compile(
            compile::<PagedDocument>(world),
            CompiledOutput::Paged,
            &mut raw_diagnostics,
        ),
        PreviewTarget::Html => collect_compile(
            compile::<HtmlDocument>(world),
            CompiledOutput::Html,
            &mut raw_diagnostics,
        ),
        PreviewTarget::Bundle => collect_compile(
            compile::<Bundle>(world),
            CompiledOutput::Bundle,
            &mut raw_diagnostics,
        ),
    };
    let compile_ms = millis(compile_started);
    let diagnostics = raw_diagnostics
        .iter()
        .map(|diagnostic| convert_diagnostic(world, diagnostic))
        .collect();

    let extraction_started = Instant::now();
    let mut equations = vec![];
    let mut pending_svg = vec![];
    for request in &params.equations {
        let source_hash = document
            .slice_chars(request.start, request.end)
            .map(|value| digest(value.as_bytes()))
            .unwrap_or_else(|_| digest(b"invalid-range"));
        let Some(compiled) = output.as_ref() else {
            equations.push(failed_result(request, source_hash, EquationStatus::Error));
            continue;
        };
        let (Ok(byte_start), Ok(byte_end)) = (
            document.char_to_byte(request.start),
            document.char_to_byte(request.end),
        ) else {
            equations.push(failed_result(request, source_hash, EquationStatus::Error));
            continue;
        };

        let frames = compiled.frames();
        let matches =
            equation_locations(world, &frames, document.source.id(), byte_start, byte_end);
        if matches.is_empty() {
            equations.push(failed_result(
                request,
                source_hash,
                EquationStatus::NotLaidOut,
            ));
            continue;
        }

        let extracted = extract_location(&frames, matches[0], Abs::pt(params.padding_pt.max(0.0)));
        let Some(mut frame) = extracted else {
            equations.push(failed_result(request, source_hash, EquationStatus::Hidden));
            continue;
        };
        let render_key = render_key(&frame.frame, &source_hash, config, params.padding_pt);
        let baseline = (!request.block).then(|| frame.baseline.to_pt());
        let width = frame.frame.width().to_pt();
        let height = frame.frame.height().to_pt();
        let index = equations.len();
        equations.push(EquationResult {
            start: request.start,
            end: request.end,
            block: request.block,
            source_hash,
            render_key: Some(render_key.clone()),
            status: EquationStatus::Ok,
            svg: None,
            width_pt: Some(width),
            height_pt: Some(height),
            baseline_pt: baseline,
            occurrence_count: matches.len(),
        });
        if !params.known_render_keys.contains(&render_key) {
            pending_svg.push((index, std::mem::take(&mut frame.frame)));
        }
    }
    let extraction_ms = millis(extraction_started);

    let svg_started = Instant::now();
    for (index, frame) in pending_svg {
        let svg = typst_svg::svg(&transparent_page(frame), &SvgOptions::default());
        equations[index].svg = Some(replace_default_fill(svg));
    }
    let svg_ms = millis(svg_started);

    RenderMathResult {
        document_version: document.version,
        source_fingerprint,
        diagnostics,
        equations,
        timings: RenderTimings {
            compile_ms,
            extraction_ms,
            svg_ms,
            total_ms: millis(total_started),
        },
    }
}

enum CompiledOutput {
    Paged(PagedDocument),
    Html(HtmlDocument),
    Bundle(Bundle),
}

impl CompiledOutput {
    fn frames(&self) -> Vec<FrameSurface<'_>> {
        let mut frames = vec![];
        match self {
            Self::Paged(document) => collect_paged_frames(document, &mut frames),
            Self::Html(document) => collect_html_frames(document.root(), &mut frames, &mut vec![]),
            Self::Bundle(bundle) => {
                for file in bundle.files.values() {
                    let BundleFile::Document(document) = file else {
                        continue;
                    };
                    match document {
                        BundleDocument::Paged(document, _) => {
                            collect_paged_frames(document, &mut frames)
                        }
                        BundleDocument::Html(document) => {
                            collect_html_frames(document.root(), &mut frames, &mut vec![])
                        }
                    }
                }
            }
        }
        frames
    }
}

#[derive(Clone, Copy)]
struct FrameSurface<'a> {
    frame: &'a Frame,
    equation: Option<&'a Content>,
}

fn collect_compile<T>(
    warned: Warned<SourceResult<T>>,
    wrap: impl FnOnce(T) -> CompiledOutput,
    diagnostics: &mut Vec<SourceDiagnostic>,
) -> Option<CompiledOutput> {
    diagnostics.extend(warned.warnings);
    match warned.output {
        Ok(output) => Some(wrap(output)),
        Err(errors) => {
            diagnostics.extend(errors);
            None
        }
    }
}

fn collect_paged_frames<'a>(document: &'a PagedDocument, frames: &mut Vec<FrameSurface<'a>>) {
    frames.extend(document.pages().iter().map(|page| FrameSurface {
        frame: &page.frame,
        equation: None,
    }));
}

fn collect_html_frames<'a>(
    element: &'a HtmlElement,
    frames: &mut Vec<FrameSurface<'a>>,
    equations: &mut Vec<&'a Content>,
) {
    for child in &element.children {
        match child {
            HtmlNode::Tag(Tag::Start(content, _))
                if content.to_packed::<EquationElem>().is_some() =>
            {
                equations.push(content)
            }
            HtmlNode::Tag(Tag::End(location, ..))
                if equations.last().and_then(|content| content.location()) == Some(*location) =>
            {
                equations.pop();
            }
            HtmlNode::Element(element) => collect_html_frames(element, frames, equations),
            HtmlNode::Frame(frame) => frames.push(FrameSurface {
                frame: &frame.inner,
                equation: equations.last().copied(),
            }),
            HtmlNode::Tag(_) | HtmlNode::Text(..) => {}
        }
    }
}

fn failed_result(
    request: &EquationRequest,
    source_hash: String,
    status: EquationStatus,
) -> EquationResult {
    EquationResult {
        start: request.start,
        end: request.end,
        block: request.block,
        source_hash,
        render_key: None,
        status,
        svg: None,
        width_pt: None,
        height_pt: None,
        baseline_pt: None,
        occurrence_count: 0,
    }
}

fn equation_locations(
    world: &LibraryWorld,
    frames: &[FrameSurface<'_>],
    file: FileId,
    start: usize,
    end: usize,
) -> Vec<Location> {
    let mut matches = vec![];
    for surface in frames {
        if let Some(content) = surface.equation
            && content.span().id() == Some(file)
            && world.range(content.span()) == Some(start..end)
        {
            matches.push(content.location().expect("tagged equation has a location"));
        }
        walk_items(surface.frame, &mut |item| {
            let FrameItem::Tag(Tag::Start(content, _)) = item else {
                return;
            };
            if content.to_packed::<EquationElem>().is_none() || content.span().id() != Some(file) {
                return;
            }
            if world.range(content.span()) == Some(start..end) {
                matches.push(content.location().expect("tagged equation has a location"));
            }
        });
    }
    matches
}

fn walk_items(frame: &Frame, visit: &mut impl FnMut(&FrameItem)) {
    for (_, item) in frame.items() {
        visit(item);
        if let FrameItem::Group(group) = item {
            walk_items(&group.frame, visit);
        }
    }
}

struct Extracted {
    frame: Frame,
    baseline: Abs,
}

struct FilterState {
    target: Location,
    active: bool,
    completed: bool,
    baseline: Option<Abs>,
}

fn extract_location(
    frames: &[FrameSurface<'_>],
    target: Location,
    padding: Abs,
) -> Option<Extracted> {
    let mut state = FilterState {
        target,
        active: false,
        completed: false,
        baseline: None,
    };
    let mut page_offset = Abs::zero();
    let total_height: Abs = frames.iter().map(|surface| surface.frame.height()).sum();
    let max_width = frames
        .iter()
        .map(|surface| surface.frame.width())
        .max()
        .unwrap_or_else(Abs::zero);
    let mut combined = Frame::soft(Size::new(max_width, total_height));

    for surface in frames {
        let transform = Transform::translate(Abs::zero(), page_offset);
        let selected_whole =
            surface.equation.and_then(|content| content.location()) == Some(target);
        let filtered = if selected_whole {
            state.baseline.get_or_insert(
                page_offset
                    + if surface.frame.has_baseline() {
                        surface.frame.baseline()
                    } else {
                        surface.frame.height()
                    },
            );
            surface.frame.clone()
        } else {
            filter_frame(surface.frame, transform, &mut state)
        };
        if !filtered.is_empty() {
            combined.push(
                Point::new(Abs::zero(), page_offset),
                FrameItem::Group(GroupItem::new(filtered)),
            );
        }
        page_offset += surface.frame.height();
    }

    let bounds = visual_bounds(&combined, Transform::identity())?;
    let offset = Point::new(padding - bounds.min.x, padding - bounds.min.y);
    combined.translate_visual(offset);
    let size = bounds.size() + Size::splat(2.0 * padding);
    combined.set_size(size);
    let baseline = state
        .baseline
        .map(|value| value - bounds.min.y + padding)
        .unwrap_or(size.y);
    combined.set_baseline(baseline);
    Some(Extracted {
        frame: combined,
        baseline,
    })
}

fn filter_frame(frame: &Frame, transform: Transform, state: &mut FilterState) -> Frame {
    let mut output = Frame::new(frame.size(), frame.kind());
    if frame.has_baseline() {
        output.set_baseline(frame.baseline());
    }

    for (position, item) in frame.items() {
        match item {
            FrameItem::Tag(tag) if tag.location() == state.target => match tag {
                Tag::Start(..) if !state.completed => {
                    state.active = true;
                    let point = position.transform(transform);
                    state.baseline.get_or_insert(point.y);
                }
                Tag::End(..) if state.active => {
                    state.active = false;
                    state.completed = true;
                }
                _ => {}
            },
            FrameItem::Group(group) if !state.completed || state.active => {
                let nested_transform = transform
                    .pre_concat(Transform::translate(position.x, position.y))
                    .pre_concat(group.transform);
                let nested = filter_frame(&group.frame, nested_transform, state);
                if !nested.is_empty() {
                    let mut selected = group.clone();
                    selected.frame = nested;
                    output.push(*position, FrameItem::Group(selected));
                }
            }
            _ if state.active && !state.completed => output.push(*position, item.clone()),
            _ => {}
        }
    }
    output
}

fn visual_bounds(frame: &Frame, transform: Transform) -> Option<Rect> {
    let mut bounds = None;
    for (position, item) in frame.items() {
        let item_bounds = match item {
            FrameItem::Text(text) => Some(text.bbox()),
            FrameItem::Shape(shape, _) => Some(shape.bbox(true)),
            FrameItem::Image(_, size, _) => Some(Rect::from_pos_size(Point::zero(), *size)),
            FrameItem::Group(group) => {
                let nested = transform
                    .pre_concat(Transform::translate(position.x, position.y))
                    .pre_concat(group.transform);
                bounds = union(bounds, visual_bounds(&group.frame, nested));
                continue;
            }
            FrameItem::Link(..) | FrameItem::Tag(..) => None,
        };
        if let Some(rect) = item_bounds {
            let translated = Rect::new(rect.min + *position, rect.max + *position);
            bounds = union(bounds, Some(transform_rect(translated, transform)));
        }
    }
    bounds
}

fn transform_rect(rect: Rect, transform: Transform) -> Rect {
    let a = rect.min.transform(transform);
    let b = Point::new(rect.max.x, rect.min.y).transform(transform);
    let c = Point::new(rect.min.x, rect.max.y).transform(transform);
    let d = rect.max.transform(transform);
    Rect::new(a.min(b).min(c).min(d), a.max(b).max(c).max(d))
}

fn union(left: Option<Rect>, right: Option<Rect>) -> Option<Rect> {
    match (left, right) {
        (None, value) | (value, None) => value,
        (Some(a), Some(b)) => Some(Rect::new(a.min.min(b.min), a.max.max(b.max))),
    }
}

fn transparent_page(frame: Frame) -> Page {
    Page {
        frame,
        bleed: Sides::splat(Abs::zero()),
        fill: Smart::Custom(None),
        numbering: None,
        supplement: Content::empty(),
        number: 1,
    }
}

fn replace_default_fill(svg: String) -> String {
    svg.replace(default_text_color().to_hex().as_str(), "currentColor")
}

fn render_key(frame: &Frame, source_hash: &str, config: &DocumentConfig, padding: f64) -> String {
    let mut hash = Sha256::new();
    hash.update(PROTOCOL_VERSION.to_le_bytes());
    hash.update(b"typst-");
    hash.update(typst_version().as_bytes());
    hash.update(hash128(frame).to_le_bytes());
    hash.update(source_hash.as_bytes());
    hash.update(padding.to_le_bytes());
    hash.update([match config.target {
        PreviewTarget::Pdf => 0,
        PreviewTarget::Html => 1,
        PreviewTarget::Bundle => 2,
    }]);
    for path in &config.font_paths {
        hash.update(path.to_string_lossy().as_bytes());
        hash.update([0]);
    }
    hash.update([1]);
    for path in &config.package_paths {
        hash.update(path.to_string_lossy().as_bytes());
        hash.update([0]);
    }
    hash.update([2]);
    if let Some(path) = &config.package_cache_path {
        hash.update(path.to_string_lossy().as_bytes());
    }
    let mut inputs: Vec<_> = config.inputs.iter().collect();
    inputs.sort_unstable_by(|left, right| left.0.cmp(right.0));
    for (key, value) in inputs {
        hash.update(key.as_bytes());
        hash.update([0]);
        hash.update(value.as_bytes());
        hash.update([0]);
    }
    hash.update([
        config.ignore_system_fonts as u8,
        config.ignore_embedded_fonts as u8,
        config.offline as u8,
    ]);
    hash.update(config.creation_timestamp.unwrap_or_default().to_le_bytes());
    let mut features: Vec<_> = config
        .features
        .iter()
        .map(|feature| match feature {
            crate::tfp_server::world::PreviewFeature::Html => 0,
            crate::tfp_server::world::PreviewFeature::Bundle => 1,
            crate::tfp_server::world::PreviewFeature::A11yExtras => 2,
        })
        .collect();
    features.sort_unstable();
    features.dedup();
    for feature in features {
        hash.update([feature]);
    }
    format!("{:x}", hash.finalize())
}

fn convert_diagnostic(world: &LibraryWorld, diagnostic: &SourceDiagnostic) -> Diagnostic {
    let id = diagnostic.span.id();
    let range = world.range(diagnostic.span);
    let source = id.and_then(|id| world.source(id).ok());
    let to_char = |byte: usize| {
        source.as_ref().map(|source| {
            source.text()[..byte.min(source.text().len())]
                .chars()
                .count()
        })
    };
    Diagnostic {
        path: id.map(|id| id.vpath().get_without_slash().to_string()),
        start: range.as_ref().and_then(|range| to_char(range.start)),
        end: range.as_ref().and_then(|range| to_char(range.end)),
        severity: match diagnostic.severity {
            Severity::Error => "error",
            Severity::Warning => "warning",
        },
        message: diagnostic.message.to_string(),
        hints: diagnostic
            .hints
            .iter()
            .map(|hint| hint.v.to_string())
            .collect(),
        trace: diagnostic
            .trace
            .iter()
            .map(|trace| format!("{:?}", trace.v))
            .collect(),
    }
}

fn digest(bytes: &[u8]) -> String {
    format!("{:x}", Sha256::digest(bytes))
}

fn millis(started: Instant) -> f64 {
    started.elapsed().as_secs_f64() * 1000.0
}

#[cfg(test)]
mod tests {
    use std::collections::HashMap;

    use typst::syntax::{RootedPath, Source, VirtualPath, VirtualRoot};

    use super::*;
    use crate::tfp_server::world::{create_project_world, load_fonts};

    fn id(path: &str) -> FileId {
        RootedPath::new(
            VirtualRoot::Project,
            VirtualPath::new(path).expect("valid test path"),
        )
        .intern()
    }

    fn render(text: &str, equations: Vec<EquationRequest>) -> RenderMathResult {
        render_target(text, equations, PreviewTarget::Pdf)
    }

    fn render_target(
        text: &str,
        equations: Vec<EquationRequest>,
        target: PreviewTarget,
    ) -> RenderMathResult {
        let directory = tempfile::tempdir().unwrap();
        let main = id("main.typ");
        let open = OpenSource::new("main.typ".into(), main, text.into(), 3);
        let config = DocumentConfig {
            target,
            ignore_system_fonts: true,
            offline: true,
            ..DocumentConfig::default()
        };
        let fonts = load_fonts(&config);
        let world = create_project_world(
            directory.path().into(),
            main,
            HashMap::from([(main, Source::new(main, text.into()))]),
            &config,
            fonts,
        )
        .unwrap();
        render_math(
            &world,
            &open,
            &config,
            &RenderMathParams {
                path: "main.typ".into(),
                version: 3,
                equations,
                padding_pt: 2.0,
                known_render_keys: HashSet::new(),
            },
        )
    }

    fn equation_request(text: &str, equation: &str, block: bool) -> EquationRequest {
        let byte_start = text.find(equation).expect("equation occurs in fixture");
        let start = text[..byte_start].chars().count();
        EquationRequest {
            start,
            end: start + equation.chars().count(),
            block,
        }
    }

    #[test]
    fn renders_inline_math_with_real_baseline_and_transparency() {
        let result = render(
            "Before $x^2$ after",
            vec![EquationRequest {
                start: 7,
                end: 12,
                block: false,
            }],
        );
        let equation = &result.equations[0];
        assert_eq!(equation.status, EquationStatus::Ok);
        assert!(equation.baseline_pt.unwrap() > 0.0);
        assert!(equation.baseline_pt.unwrap() < equation.height_pt.unwrap());
        let svg = equation.svg.as_ref().unwrap();
        assert!(svg.contains("currentColor"), "{svg}");
        assert!(!svg.contains("fill=\"#ffffff\""), "{svg}");
    }

    #[test]
    fn renders_block_math() {
        let text = "$ x + y = z $";
        let result = render(
            text,
            vec![EquationRequest {
                start: 0,
                end: text.chars().count(),
                block: true,
            }],
        );
        assert_eq!(result.equations[0].status, EquationStatus::Ok);
        assert!(result.equations[0].baseline_pt.is_none());
        assert!(result.equations[0].svg.is_some());
    }

    #[test]
    fn renders_html_target_equations_with_html_semantics() {
        let text =
            "#context if target() == \"html\" { [Before $x^2$ after] } else { [wrong target] }";
        let result = render_target(
            text,
            vec![equation_request(text, "$x^2$", false)],
            PreviewTarget::Html,
        );
        assert!(
            result
                .diagnostics
                .iter()
                .all(|diagnostic| diagnostic.severity != "error"),
            "{:?}",
            result.diagnostics
        );
        let equation = &result.equations[0];
        assert_eq!(equation.status, EquationStatus::Ok);
        assert!(equation.baseline_pt.is_some());
        assert!(equation.svg.as_ref().unwrap().contains("currentColor"));
    }

    #[test]
    fn renders_html_block_equations_and_user_show_rules() {
        let text = "#show math.equation: set text(fill: red)\nBefore\n$ x + y $\nAfter";
        let result = render_target(
            text,
            vec![equation_request(text, "$ x + y $", true)],
            PreviewTarget::Html,
        );
        let equation = &result.equations[0];
        assert_eq!(equation.status, EquationStatus::Ok);
        assert!(equation.baseline_pt.is_none());
        let svg = equation.svg.as_ref().unwrap();
        assert!(svg.contains("#ff4136"), "{svg}");
        assert!(!svg.contains("currentColor"), "{svg}");
    }

    #[test]
    fn counts_repeated_html_target_equations() {
        let text = "#let repeated = [$x$]\n#repeated #repeated";
        let result = render_target(
            text,
            vec![equation_request(text, "$x$", false)],
            PreviewTarget::Html,
        );
        assert_eq!(result.equations[0].status, EquationStatus::Ok);
        assert_eq!(result.equations[0].occurrence_count, 2);
    }

    #[test]
    fn renders_paged_and_html_documents_in_bundle_target() {
        let text = concat!(
            "#document(\"paper.pdf\")[Paged $p^2$]\n",
            "#document(\"index.html\")[HTML $h^2$]\n"
        );
        let result = render_target(
            text,
            vec![
                equation_request(text, "$p^2$", false),
                equation_request(text, "$h^2$", false),
            ],
            PreviewTarget::Bundle,
        );
        assert!(
            result
                .diagnostics
                .iter()
                .all(|diagnostic| diagnostic.severity != "error"),
            "{:?}",
            result.diagnostics
        );
        assert_eq!(result.equations.len(), 2);
        for equation in &result.equations {
            assert_eq!(equation.status, EquationStatus::Ok);
            assert!(equation.svg.is_some());
            assert!(equation.baseline_pt.is_some());
        }
    }

    #[test]
    fn preserves_authored_colors() {
        let text = "#text(fill: red)[$x$]";
        let result = render(
            text,
            vec![EquationRequest {
                start: 17,
                end: 20,
                block: false,
            }],
        );
        let svg = result.equations[0].svg.as_ref().unwrap();
        assert!(svg.contains("#ff4136"), "{svg}");
        assert!(!svg.contains("currentColor"), "{svg}");
    }

    #[test]
    fn reports_repeated_equation_occurrences() {
        let text = "#let repeated = [$x$]\n#repeated #repeated";
        let result = render(
            text,
            vec![EquationRequest {
                start: 17,
                end: 20,
                block: false,
            }],
        );
        assert_eq!(result.equations[0].status, EquationStatus::Ok);
        assert_eq!(result.equations[0].occurrence_count, 2);
    }

    #[test]
    fn hidden_equation_is_not_approximated() {
        let text = "#if false [$x$]";
        let result = render(
            text,
            vec![EquationRequest {
                start: 11,
                end: 14,
                block: false,
            }],
        );
        assert_eq!(result.equations[0].status, EquationStatus::NotLaidOut);
        assert!(result.equations[0].svg.is_none());
    }

    #[test]
    fn reports_compile_errors_without_approximate_output() {
        let text = "$unknown_function($";
        let result = render(
            text,
            vec![EquationRequest {
                start: 0,
                end: text.chars().count(),
                block: false,
            }],
        );
        assert_eq!(result.equations[0].status, EquationStatus::Error);
        assert!(!result.diagnostics.is_empty());
    }
}
