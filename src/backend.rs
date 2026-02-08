use std::cell::RefCell;
use std::collections::{HashMap, HashSet};
use std::fs;
use std::path::{Path, PathBuf};

use crate::error::StrResult;
use ecow::eco_format;
use ego_tree::{NodeId, NodeRef};
use html5ever::LocalName;
use scraper::{Html, Node, Selector};
use serde::Serialize;
use tera::{Context, Error as TeraError, Tera, Value as TeraValue};

use crate::config::{BuildConfig, SiteSettings};
use crate::html::{HtmlNote, add_class_to_element};

struct Note {
    id: String,
    path: PathBuf,
    document: Html,
    transcludes: Vec<String>,
    links_out: Vec<String>,
}

struct ProcessedNote {
    head_html: String,
    body_html: String,
    metadata: HashMap<String, String>,
    title: Option<String>,
}

struct RenderedNote {
    body_html: String,
    citations: Vec<String>,
    related: Vec<String>,
}

trait TransclusionLookup {
    fn body_html(&self, id: &str) -> Option<&str>;
}

impl TransclusionLookup for HashMap<String, ProcessedNote> {
    fn body_html(&self, id: &str) -> Option<&str> {
        self.get(id).map(|note| note.body_html.as_str())
    }
}

impl TransclusionLookup for HashMap<String, RenderedNote> {
    fn body_html(&self, id: &str) -> Option<&str> {
        self.get(id).map(|note| note.body_html.as_str())
    }
}

#[derive(Serialize)]
struct Heading {
    level: u8,
    id: String,
    content: String,
    disable_numbering: bool,
    children: Vec<Heading>,
}

#[derive(Serialize)]
struct BackmatterSection {
    title: String,
    content: String,
}

#[derive(Serialize)]
struct NoteTemplateContext<'a> {
    id: &'a str,
    title: Option<&'a str>,
    metadata: &'a HashMap<String, String>,
    head: &'a str,
    content: &'a str,
    backmatter_sections: Vec<BackmatterSection>,
    toc: &'a [Heading],
}

#[derive(Serialize)]
struct LinkTemplateContext<'a> {
    target: &'a str,
    text: &'a str,
    href: &'a str,
}

#[derive(Serialize)]
struct CitationTemplateContext<'a> {
    target: &'a str,
    text: &'a str,
    href: &'a str,
}

#[derive(Serialize)]
struct TransclusionTemplateContext<'a> {
    target: &'a str,
    show_metadata: bool,
    expanded: bool,
    disable_numbering: bool,
    demote_headings: usize,
    content: &'a str,
}

#[derive(Serialize)]
struct SiteTemplateContext<'a> {
    root_dir: &'a str,
    trailing_slash: bool,
    domain: Option<&'a str>,
}

pub fn process_html(build_config: &BuildConfig, html_notes: Vec<HtmlNote>) -> StrResult<()> {
    let public_dir = &build_config.public_directory;
    let output_dir = &build_config.output_directory;
    let templates = load_templates()?;

    let notes = load_notes(html_notes)?;
    let order = topo_sort_transclusions(&notes)?;

    let note_ids: HashSet<String> = notes.keys().cloned().collect();
    let mut processed_notes = HashMap::new();

    for note_id in &order {
        let note = notes
            .get(note_id)
            .ok_or_else(|| eco_format!("missing note {note_id} during processing"))?;

        let processed_note = process_note(note, &processed_notes, &build_config.site, &templates)?;
        processed_notes.insert(note_id.clone(), processed_note);
    }

    let backlinks = compute_backlinks(&notes);
    let contexts = compute_contexts(&notes);
    let transcluded_descendants = compute_transcluded_descendants(&notes, &order);
    let mut rendered_notes = HashMap::new();
    for note_id in &order {
        let note = notes
            .get(note_id)
            .ok_or_else(|| eco_format!("missing note {note_id} during rendering"))?;
        let processed = processed_notes
            .get(note_id)
            .ok_or_else(|| eco_format!("missing processed note for {note_id}"))?;
        let (body_html, mut citations, mut related) = render_links_in_body(
            processed.body_html.as_str(),
            Some(note.path.as_path()),
            &note_ids,
            &processed_notes,
            &build_config.site,
            &templates,
        )?;
        if let Some(excluded) = transcluded_descendants.get(note_id) {
            citations.retain(|id| !excluded.contains(id));
            related.retain(|id| !excluded.contains(id));
        }
        rendered_notes.insert(
            note_id.clone(),
            RenderedNote {
                body_html,
                citations,
                related,
            },
        );
    }

    fs::create_dir_all(output_dir).map_err(|err| {
        eco_format!(
            "failed to create output directory {}: {err}",
            output_dir.display()
        )
    })?;

    if public_dir.exists() {
        copy_dir_all(public_dir, output_dir)?;
    }

    for (note_id, note) in &notes {
        let processed = processed_notes
            .get(note_id)
            .ok_or_else(|| eco_format!("missing processed note for {note_id}"))?;
        let rendered = rendered_notes
            .get(note_id)
            .ok_or_else(|| eco_format!("missing rendered note for {note_id}"))?;
        let backmatter_sections = build_backmatter_sections(
            note_id,
            &backlinks,
            &contexts,
            rendered.citations.as_slice(),
            rendered.related.as_slice(),
            &rendered_notes,
            &templates,
            &build_config.site,
        )?;
        let toc = build_toc(rendered.body_html.as_str())?;

        let note_context = NoteTemplateContext {
            id: note_id.as_str(),
            title: processed.title.as_deref(),
            metadata: &processed.metadata,
            head: processed.head_html.as_str(),
            content: rendered.body_html.as_str(),
            backmatter_sections,
            toc: &toc,
        };
        let site_context = site_template_context(&build_config.site);
        let mut context = Context::new();
        context.insert("note", &note_context);
        context.insert("site", &site_context);
        let final_html = render_template(&templates, "note.html", &context)?;

        let output_path = output_path_for_note(output_dir, &note.id, &build_config.site);
        if let Some(parent) = output_path.parent() {
            fs::create_dir_all(parent).map_err(|err| {
                eco_format!(
                    "failed to create output directory {}: {err}",
                    parent.display()
                )
            })?;
        }
        fs::write(&output_path, final_html).map_err(|err| {
            eco_format!(
                "failed to write output file {}: {err}",
                output_path.display()
            )
        })?;
    }

    Ok(())
}

fn process_note(
    note: &Note,
    processed_notes: &HashMap<String, ProcessedNote>,
    site: &SiteSettings,
    templates: &Tera,
) -> StrResult<ProcessedNote> {
    let body_html = render_note_body(note, processed_notes, site, templates)?;
    let head_html = render_note_head(note)?;
    let metadata = crate::html::extract_metadata(&note.document)?;
    let title = crate::html::extract_note_title(&note.document, &metadata)?;
    Ok(ProcessedNote {
        head_html,
        body_html,
        metadata,
        title,
    })
}

fn load_templates() -> StrResult<Tera> {
    let pattern = ".wb/templates/**/*.html";
    let mut tera = Tera::new(pattern)
        .map_err(|err| eco_format!("failed to load templates from {pattern}: {err}"))?;
    tera.register_filter("wb_disable_numbering", wb_disable_numbering_filter);
    tera.register_filter("wb_demote_headings", wb_demote_headings_filter);
    Ok(tera)
}

fn render_template(templates: &Tera, name: &str, context: &Context) -> StrResult<String> {
    templates
        .render(name, context)
        .map_err(|err| eco_format!("failed to render template {name}: {err}"))
}

fn site_template_context(site: &SiteSettings) -> SiteTemplateContext<'_> {
    SiteTemplateContext {
        root_dir: site.root_dir.as_str(),
        trailing_slash: site.trailing_slash,
        domain: site.domain.as_deref(),
    }
}

fn render_internal_link(
    templates: &Tera,
    site: &SiteSettings,
    target: &str,
    text: &str,
) -> StrResult<String> {
    let href = build_note_href(target, site);
    let link = LinkTemplateContext {
        target,
        text,
        href: href.as_str(),
    };
    let site_context = site_template_context(site);
    let mut context = Context::new();
    context.insert("link", &link);
    context.insert("site", &site_context);
    render_template(templates, "internal_link.html", &context)
}

fn render_citation(
    templates: &Tera,
    site: &SiteSettings,
    target: &str,
    text: &str,
) -> StrResult<String> {
    let href = build_note_href(target, site);
    let citation = CitationTemplateContext {
        target,
        text,
        href: href.as_str(),
    };
    let site_context = site_template_context(site);
    let mut context = Context::new();
    context.insert("citation", &citation);
    context.insert("site", &site_context);
    render_template(templates, "citation.html", &context)
}

fn render_transclusion(
    templates: &Tera,
    site: &SiteSettings,
    transclusion: &TransclusionTemplateContext,
) -> StrResult<String> {
    let site_context = site_template_context(site);
    let mut context = Context::new();
    context.insert("transclusion", transclusion);
    context.insert("site", &site_context);
    render_template(templates, "transclusion.html", &context)
}

fn prepare_transclusion_content(body_html: &str) -> StrResult<String> {
    Ok(body_html.to_string())
}

fn load_notes(html_notes: Vec<HtmlNote>) -> StrResult<HashMap<String, Note>> {
    let mut notes = HashMap::new();

    for note in html_notes {
        if notes.contains_key(&note.id) {
            return Err(eco_format!(
                "duplicate note id {} found while reading {}",
                note.id,
                note.source_path.display()
            ));
        }

        let transcludes =
            crate::html::collect_targets(&note.document, "wb-transclusion", &note.source_path)?;
        let links_out =
            crate::html::collect_targets(&note.document, "wb-internal-link", &note.source_path)?;

        notes.insert(
            note.id.clone(),
            Note {
                id: note.id,
                path: note.source_path,
                document: note.document,
                transcludes,
                links_out,
            },
        );
    }

    if notes.is_empty() {
        return Err(eco_format!("no html notes provided to backend"));
    }

    Ok(notes)
}

fn topo_sort_transclusions(notes: &HashMap<String, Note>) -> StrResult<Vec<String>> {
    let mut graph = HashMap::new();
    for (id, note) in notes {
        let mut targets = Vec::new();
        for target in &note.transcludes {
            if !notes.contains_key(target) {
                return Err(eco_format!(
                    "transclusion target {target} referenced by {id} does not exist"
                ));
            }
            targets.push(target.clone());
        }
        graph.insert(id.clone(), targets);
    }

    let mut order = Vec::with_capacity(notes.len());
    let mut state: HashMap<String, VisitState> = HashMap::new();
    let mut stack: Vec<String> = Vec::new();

    for id in notes.keys() {
        if !state.contains_key(id) {
            visit(id, &graph, &mut state, &mut order, &mut stack)?;
        }
    }

    Ok(order)
}

#[derive(Copy, Clone, PartialEq, Eq)]
enum VisitState {
    Visiting,
    Done,
}

fn visit(
    id: &str,
    graph: &HashMap<String, Vec<String>>,
    state: &mut HashMap<String, VisitState>,
    order: &mut Vec<String>,
    stack: &mut Vec<String>,
) -> StrResult<()> {
    if let Some(existing) = state.get(id) {
        if *existing == VisitState::Visiting {
            stack.push(id.to_string());
            return Err(eco_format!(
                "transclusion cycle detected: {}",
                stack.join(" -> ")
            ));
        }
        return Ok(());
    }

    state.insert(id.to_string(), VisitState::Visiting);
    stack.push(id.to_string());

    if let Some(neighbors) = graph.get(id) {
        for target in neighbors {
            visit(target, graph, state, order, stack)?;
        }
    }

    stack.pop();
    state.insert(id.to_string(), VisitState::Done);
    order.push(id.to_string());
    Ok(())
}

fn render_note_body(
    note: &Note,
    transclusion_lookup: &dyn TransclusionLookup,
    site: &SiteSettings,
    templates: &Tera,
) -> StrResult<String> {
    let selector = Selector::parse("body")
        .map_err(|err| eco_format!("failed to parse selector body: {err}"))?;
    let body = note
        .document
        .select(&selector)
        .next()
        .ok_or_else(|| eco_format!("missing <body> in {}", note.path.display()))?;

    let context = RenderContext {
        mode: RenderMode::Transclusion {
            transclusion_lookup,
            site,
            templates,
        },
        note_path: Some(&note.path),
    };

    render_children(*body, &context)
}

fn render_links_in_body(
    body_html: &str,
    note_path: Option<&Path>,
    note_ids: &HashSet<String>,
    processed_notes: &HashMap<String, ProcessedNote>,
    site: &SiteSettings,
    templates: &Tera,
) -> StrResult<(String, Vec<String>, Vec<String>)> {
    let fragment = Html::parse_fragment(body_html);
    let citations = RefCell::new(HashSet::new());
    let related = RefCell::new(HashSet::new());
    let context = RenderContext {
        mode: RenderMode::Links {
            note_ids,
            processed_notes,
            site,
            templates,
            citations: Some(&citations),
            related: Some(&related),
        },
        note_path,
    };
    let rendered = render_children(fragment.tree.root(), &context)?;
    let mut citations: Vec<String> = citations.into_inner().into_iter().collect();
    citations.sort();
    let mut related: Vec<String> = related.into_inner().into_iter().collect();
    related.sort();
    Ok((rendered, citations, related))
}

fn render_note_head(note: &Note) -> StrResult<String> {
    let selector = Selector::parse("head")
        .map_err(|err| eco_format!("failed to parse selector head: {err}"))?;
    let head = note
        .document
        .select(&selector)
        .next()
        .ok_or_else(|| eco_format!("missing <head> in {}", note.path.display()))?;

    let context = RenderContext {
        mode: RenderMode::Fragment,
        note_path: None,
    };

    render_children(*head, &context)
}

struct HeadingRef {
    path: Vec<usize>,
    level: u8,
}

fn build_toc(body_html: &str) -> StrResult<Vec<Heading>> {
    let mut toc: Vec<Heading> = Vec::new();
    let mut stack: Vec<HeadingRef> = Vec::new();

    let context = RenderContext {
        mode: RenderMode::Fragment,
        note_path: None,
    };

    let selector = Selector::parse("h1, h2, h3, h4, h5, h6")
        .map_err(|err| eco_format!("failed to parse selector for headings: {err}"))?;
    let fragment = Html::parse_fragment(body_html);
    for heading in fragment.select(&selector) {
        let element = heading.value();
        let level = heading_level(element.name()).ok_or_else(|| {
            eco_format!("invalid heading tag {} in TOC generation", element.name())
        })?;
        let id = element.attr("id").unwrap_or("").to_string();
        let content = render_children(*heading, &context)?;
        let disable_numbering = element
            .attr("class")
            .map(|class| class.split_whitespace().any(|c| c == "disable-numbering"))
            .unwrap_or(false);
        let heading = Heading {
            level,
            id,
            content,
            disable_numbering,
            children: Vec::new(),
        };

        while let Some(last) = stack.last() {
            if level > last.level {
                break;
            }
            stack.pop();
        }

        let parent_path = stack.last().map(|item| item.path.as_slice());
        let path = push_heading(&mut toc, parent_path, heading)?;
        stack.push(HeadingRef { path, level });
    }

    Ok(toc)
}

fn push_heading(
    toc: &mut Vec<Heading>,
    parent_path: Option<&[usize]>,
    heading: Heading,
) -> StrResult<Vec<usize>> {
    if let Some(path) = parent_path {
        let parent =
            get_heading_mut(toc, path).ok_or_else(|| eco_format!("invalid toc heading path"))?;
        parent.children.push(heading);
        let mut child_path = path.to_vec();
        child_path.push(parent.children.len() - 1);
        Ok(child_path)
    } else {
        toc.push(heading);
        Ok(vec![toc.len() - 1])
    }
}

fn get_heading_mut<'a>(headings: &'a mut [Heading], path: &[usize]) -> Option<&'a mut Heading> {
    let (index, rest) = path.split_first()?;
    let heading = headings.get_mut(*index)?;
    if rest.is_empty() {
        return Some(heading);
    }
    get_heading_mut(&mut heading.children, rest)
}

fn compute_backlinks(notes: &HashMap<String, Note>) -> HashMap<String, Vec<String>> {
    compute_reverse_index(notes, |note| &note.links_out)
}

fn compute_contexts(notes: &HashMap<String, Note>) -> HashMap<String, Vec<String>> {
    compute_reverse_index(notes, |note| &note.transcludes)
}

fn compute_reverse_index<F>(notes: &HashMap<String, Note>, edges: F) -> HashMap<String, Vec<String>>
where
    F: Fn(&Note) -> &[String],
{
    let mut index: HashMap<String, HashSet<String>> = HashMap::new();
    for (source_id, note) in notes {
        for target in edges(note) {
            index
                .entry(target.clone())
                .or_default()
                .insert(source_id.clone());
        }
    }
    index
        .into_iter()
        .map(|(key, value)| (key, value.into_iter().collect()))
        .collect()
}

fn compute_transcluded_descendants(
    notes: &HashMap<String, Note>,
    order: &[String],
) -> HashMap<String, HashSet<String>> {
    let mut descendants: HashMap<String, HashSet<String>> = HashMap::new();
    for id in order {
        let note = match notes.get(id) {
            Some(note) => note,
            None => continue,
        };
        let mut set = HashSet::new();
        for target in &note.transcludes {
            set.insert(target.clone());
            if let Some(child_set) = descendants.get(target) {
                set.extend(child_set.iter().cloned());
            }
        }
        descendants.insert(id.clone(), set);
    }
    descendants
}

#[allow(clippy::too_many_arguments)]
fn build_backmatter_sections(
    note_id: &str,
    backlinks: &HashMap<String, Vec<String>>,
    contexts: &HashMap<String, Vec<String>>,
    references: &[String],
    related: &[String],
    transclusion_lookup: &dyn TransclusionLookup,
    templates: &Tera,
    site: &SiteSettings,
) -> StrResult<Vec<BackmatterSection>> {
    let mut sections = vec![];
    if let Some(ids) = contexts.get(note_id) {
        let section =
            render_backmatter_section("Contexts", ids, transclusion_lookup, templates, site)?;
        sections.push(section);
    }
    if !references.is_empty() {
        let section = render_backmatter_section(
            "References",
            references,
            transclusion_lookup,
            templates,
            site,
        )?;
        sections.push(section);
    }
    if let Some(ids) = backlinks.get(note_id) {
        let section =
            render_backmatter_section("Backlinks", ids, transclusion_lookup, templates, site)?;
        sections.push(section);
    }
    if !related.is_empty() {
        let section =
            render_backmatter_section("Related", related, transclusion_lookup, templates, site)?;
        sections.push(section);
    }
    Ok(sections)
}

fn render_backmatter_section(
    title: &str,
    included_note_ids: &[String],
    transclusion_lookup: &dyn TransclusionLookup,
    templates: &Tera,
    site: &SiteSettings,
) -> StrResult<BackmatterSection> {
    if included_note_ids.is_empty() {
        return Ok(BackmatterSection {
            title: title.to_string(),
            content: String::new(),
        });
    }

    let mut included_ids = included_note_ids.to_vec();
    included_ids.sort();

    let virtual_note_body = included_ids
        .iter()
        .map(|id| {
            String::new()
                + "<wb-transclusion target=\""
                + id.as_str()
                + "\" show-metadata=\"true\" expanded=\"false\" hide-numbering=\"true\" demote-headings=\"1\"></wb-transclusion>"
        })
        .collect::<String>();

    let virtual_note = Note {
        id: String::new(),    // unused
        path: PathBuf::new(), // shouldn't be used
        document: Html::parse_document(&format!(
            "<html><head></head><body>{}</body></html>",
            virtual_note_body
        )),
        transcludes: Vec::new(), // unused
        links_out: included_ids, // unused
    };

    let body_html = render_note_body(&virtual_note, transclusion_lookup, site, templates)?;

    let section = BackmatterSection {
        title: title.to_string(),
        content: body_html,
    };

    Ok(section)
}

struct RenderContext<'a> {
    mode: RenderMode<'a>,
    note_path: Option<&'a Path>,
}

enum RenderMode<'a> {
    Transclusion {
        transclusion_lookup: &'a dyn TransclusionLookup,
        site: &'a SiteSettings,
        templates: &'a Tera,
    },
    Links {
        note_ids: &'a HashSet<String>,
        processed_notes: &'a HashMap<String, ProcessedNote>,
        site: &'a SiteSettings,
        templates: &'a Tera,
        citations: Option<&'a RefCell<HashSet<String>>>,
        related: Option<&'a RefCell<HashSet<String>>>,
    },
    Fragment,
}

fn render_node(node: NodeRef<Node>, context: &RenderContext) -> StrResult<String> {
    match node.value() {
        Node::Document | Node::Fragment => render_children(node, context),
        Node::Doctype(doctype) => Ok(render_doctype(doctype)),
        Node::Comment(comment) => Ok(format!("<!--{}-->", &**comment)),
        Node::Text(text) => Ok(escape_text(text)),
        Node::ProcessingInstruction(pi) => Ok(format!("<?{} {}?>", pi.target, pi.data)),
        Node::Element(element) => render_element(node, element, context),
    }
}

fn render_children(node: NodeRef<Node>, context: &RenderContext) -> StrResult<String> {
    let mut out = String::new();
    for child in node.children() {
        out.push_str(&render_node(child, context)?);
    }
    Ok(out)
}

fn render_raw_children(node: NodeRef<Node>, context: &RenderContext) -> StrResult<String> {
    let mut out = String::new();
    for child in node.children() {
        match child.value() {
            Node::Text(text) => out.push_str(text),
            Node::Comment(comment) => out.push_str(&format!("<!--{}-->", &**comment)),
            _ => out.push_str(&render_node(child, context)?),
        }
    }
    Ok(out)
}

fn render_element(
    node: NodeRef<Node>,
    element: &scraper::node::Element,
    context: &RenderContext,
) -> StrResult<String> {
    let tag = element.name();

    match &context.mode {
        RenderMode::Transclusion {
            transclusion_lookup,
            site,
            templates,
        } => {
            if tag.eq_ignore_ascii_case("wb-transclusion") {
                let target_raw = element.attr("target").ok_or_else(|| {
                    eco_format!(
                        "wb-transclusion missing target in {}",
                        path_display(context)
                    )
                })?;
                let target = crate::html::normalize_target(target_raw);
                let body_html = transclusion_lookup.body_html(&target).ok_or_else(|| {
                    eco_format!(
                        "transclusion target {target} referenced by {} is not processed yet",
                        path_display(context)
                    )
                })?;
                let show_metadata =
                    crate::html::parse_bool_attr(element.attr("show-metadata"), true);
                let expanded = crate::html::parse_bool_attr(element.attr("expanded"), true);
                let disable_numbering =
                    crate::html::parse_bool_attr(element.attr("disable-numbering"), false);
                let demote_headings =
                    crate::html::parse_non_negative_usize_attr(element.attr("demote-headings"), 1);
                let content_html = prepare_transclusion_content(body_html)?;
                let transclusion = TransclusionTemplateContext {
                    target: target.as_str(),
                    show_metadata,
                    expanded,
                    disable_numbering,
                    demote_headings,
                    content: content_html.as_str(),
                };
                return render_transclusion(templates, site, &transclusion);
            }
        }
        RenderMode::Links {
            note_ids,
            processed_notes,
            site,
            templates,
            citations,
            related,
        } => {
            if tag.eq_ignore_ascii_case("wb-transclusion") {
                return Err(eco_format!("unexpected wb-transclusion in link rendering"));
            }
            if tag.eq_ignore_ascii_case("wb-internal-link") || tag.eq_ignore_ascii_case("wb-cite") {
                let target_raw = element.attr("target").ok_or_else(|| {
                    eco_format!("{} missing target in {}", tag, path_display(context))
                })?;
                let target = crate::html::normalize_target(target_raw);
                if !note_ids.contains(&target) {
                    return Err(eco_format!(
                        "link target {target} referenced by {} does not exist",
                        path_display(context)
                    ));
                }
                if tag.eq_ignore_ascii_case("wb-cite")
                    && let Some(citations) = citations
                {
                    citations.borrow_mut().insert(target.clone());
                }
                if tag.eq_ignore_ascii_case("wb-internal-link")
                    && let Some(related) = related
                {
                    related.borrow_mut().insert(target.clone());
                }
                let mut content = render_children(node, context)?;
                if content.is_empty()
                    && let Some(title) = processed_notes
                        .get(&target)
                        .and_then(|note| note.title.as_deref())
                {
                    content = escape_text(title);
                }
                if tag.eq_ignore_ascii_case("wb-cite") {
                    return render_citation(templates, site, &target, &content);
                }
                return render_internal_link(templates, site, &target, &content);
            }
        }
        RenderMode::Fragment => {
            if tag.eq_ignore_ascii_case("wb-transclusion") {
                return Err(eco_format!("unexpected wb- element in processed fragment"));
            }
        }
    }

    let (attrs, is_void) = build_attributes(element);

    if tag.eq_ignore_ascii_case("script") || tag.eq_ignore_ascii_case("style") {
        let mut out = String::new();
        out.push('<');
        out.push_str(tag);
        out.push_str(&attrs);
        out.push('>');
        out.push_str(&render_raw_children(node, context)?);
        out.push_str("</");
        out.push_str(tag);
        out.push('>');
        return Ok(out);
    }

    let mut out = String::new();
    out.push('<');
    out.push_str(tag);
    out.push_str(&attrs);

    if is_void {
        out.push('>');
        return Ok(out);
    }

    out.push('>');
    out.push_str(&render_children(node, context)?);
    out.push_str("</");
    out.push_str(tag);
    out.push('>');

    Ok(out)
}

fn build_attributes(element: &scraper::node::Element) -> (String, bool) {
    let is_void = is_void_element(element.name());

    let mut out = String::new();
    for (name, value) in element.attrs() {
        out.push(' ');
        out.push_str(name);
        out.push_str("=\"");
        out.push_str(&escape_attr(value));
        out.push('"');
    }

    (out, is_void)
}

fn render_fragment(root: NodeRef<Node>) -> StrResult<String> {
    let context = RenderContext {
        mode: RenderMode::Fragment,
        note_path: None,
    };
    render_children(root, &context)
}

fn with_element_mut<F>(fragment: &mut Html, node_id: NodeId, f: F)
where
    F: FnOnce(&mut scraper::node::Element),
{
    if let Some(mut node) = fragment.tree.get_mut(node_id)
        && let Node::Element(element) = node.value()
    {
        f(element);
    }
}

fn wb_disable_numbering_filter(
    value: &TeraValue,
    _args: &HashMap<String, TeraValue>,
) -> tera::Result<TeraValue> {
    let html = value
        .as_str()
        .ok_or_else(|| TeraError::msg("wb_disable_numbering expects a string value"))?;
    let mut fragment = Html::parse_fragment(html);
    let headings = {
        let root = fragment.tree.root();
        let mut headings = Vec::new();
        for node in root.descendants() {
            if let Some(element) = node.value().as_element()
                && is_heading_tag(element.name())
            {
                headings.push(node.id());
            }
        }
        headings
    };
    for node_id in headings {
        with_element_mut(&mut fragment, node_id, |element| {
            add_class_to_element(element, "disable-numbering");
        });
    }
    let rendered =
        render_fragment(fragment.tree.root()).map_err(|err| TeraError::msg(err.to_string()))?;
    Ok(TeraValue::String(rendered))
}

fn wb_demote_headings_filter(
    value: &TeraValue,
    args: &HashMap<String, TeraValue>,
) -> tera::Result<TeraValue> {
    let html = value
        .as_str()
        .ok_or_else(|| TeraError::msg("wb_demote_headings expects a string value"))?;
    let levels = parse_demote_levels(args)?;
    if levels == 0 {
        return Ok(TeraValue::String(html.to_string()));
    }
    let mut fragment = Html::parse_fragment(html);
    let headings = {
        let root = fragment.tree.root();
        let mut headings = Vec::new();
        for node in root.descendants() {
            if let Some(element) = node.value().as_element()
                && let Some(demoted) = demote_heading_tag(element.name(), levels)
                && !element.name().eq_ignore_ascii_case(demoted)
            {
                headings.push((node.id(), LocalName::from(demoted)));
            }
        }
        headings
    };
    for (node_id, demoted) in headings {
        with_element_mut(&mut fragment, node_id, |element| {
            element.name.local = demoted;
        });
    }
    let rendered =
        render_fragment(fragment.tree.root()).map_err(|err| TeraError::msg(err.to_string()))?;
    Ok(TeraValue::String(rendered))
}

fn parse_demote_levels(args: &HashMap<String, TeraValue>) -> tera::Result<usize> {
    let Some(levels) = args.get("levels") else {
        return Ok(1);
    };
    if let Some(levels) = levels.as_u64() {
        return usize::try_from(levels)
            .map_err(|_| TeraError::msg("wb_demote_headings levels value is too large"));
    }
    if let Some(levels) = levels.as_i64() {
        if levels < 0 {
            return Err(TeraError::msg(
                "wb_demote_headings levels must be a non-negative integer",
            ));
        }
        return usize::try_from(levels as u64)
            .map_err(|_| TeraError::msg("wb_demote_headings levels value is too large"));
    }
    Err(TeraError::msg(
        "wb_demote_headings expects `levels` to be a non-negative integer",
    ))
}

fn demote_heading_tag(tag: &str, levels: usize) -> Option<&'static str> {
    let level = heading_level(tag)?;
    let demoted_level = (usize::from(level) + levels).min(6) as u8;
    heading_tag(demoted_level)
}

fn heading_tag(level: u8) -> Option<&'static str> {
    match level {
        1 => Some("h1"),
        2 => Some("h2"),
        3 => Some("h3"),
        4 => Some("h4"),
        5 => Some("h5"),
        6 => Some("h6"),
        _ => None,
    }
}

fn heading_level(tag: &str) -> Option<u8> {
    match tag.to_ascii_lowercase().as_str() {
        "h1" => Some(1),
        "h2" => Some(2),
        "h3" => Some(3),
        "h4" => Some(4),
        "h5" => Some(5),
        "h6" => Some(6),
        _ => None,
    }
}

fn is_heading_tag(tag: &str) -> bool {
    matches!(
        tag.to_ascii_lowercase().as_str(),
        "h1" | "h2" | "h3" | "h4" | "h5" | "h6"
    )
}

fn build_note_href(note_id: &str, site: &SiteSettings) -> String {
    if note_id == "index" {
        return site.root_dir.clone();
    }
    if site.trailing_slash {
        format!("{}{note_id}/", site.root_dir)
    } else {
        format!("{}{note_id}.html", site.root_dir)
    }
}

fn output_path_for_note(output_dir: &Path, note_id: &str, site: &SiteSettings) -> PathBuf {
    if note_id == "index" {
        return output_dir.join("index.html");
    }
    if site.trailing_slash {
        output_dir.join(note_id).join("index.html")
    } else {
        output_dir.join(format!("{note_id}.html"))
    }
}

fn is_void_element(tag: &str) -> bool {
    matches!(
        tag,
        "area"
            | "base"
            | "br"
            | "col"
            | "embed"
            | "hr"
            | "img"
            | "input"
            | "link"
            | "meta"
            | "param"
            | "source"
            | "track"
            | "wbr"
    )
}

fn escape_text(text: &str) -> String {
    text.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
}

fn escape_attr(value: &str) -> String {
    value
        .replace('&', "&amp;")
        .replace('"', "&quot;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
}

fn render_doctype(doctype: &scraper::node::Doctype) -> String {
    if doctype.public_id().is_empty() && doctype.system_id().is_empty() {
        return format!("<!DOCTYPE {}>", doctype.name());
    }
    format!(
        "<!DOCTYPE {} PUBLIC \"{}\" \"{}\">",
        doctype.name(),
        doctype.public_id(),
        doctype.system_id()
    )
}

fn path_display(context: &RenderContext) -> String {
    context
        .note_path
        .map(|path| path.display().to_string())
        .unwrap_or_else(|| "<unknown>".to_string())
}

fn copy_dir_all(src: &Path, dst: &Path) -> StrResult<()> {
    if !dst.exists() {
        fs::create_dir_all(dst)
            .map_err(|err| eco_format!("failed to create directory {}: {err}", dst.display()))?;
    }

    for entry in fs::read_dir(src)
        .map_err(|err| eco_format!("failed to read directory {}: {err}", src.display()))?
    {
        let entry = entry.map_err(|err| eco_format!("failed to read directory entry: {err}"))?;
        let path = entry.path();
        let target = dst.join(entry.file_name());
        if path.is_dir() {
            copy_dir_all(&path, &target)?;
        } else {
            fs::copy(&path, &target).map_err(|err| {
                eco_format!(
                    "failed to copy {} to {}: {err}",
                    path.display(),
                    target.display()
                )
            })?;
        }
    }

    Ok(())
}
