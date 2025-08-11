use std::collections::{HashMap, HashSet};
use std::fs;
use std::path::{Path, PathBuf};

use ecow::eco_format;
use ego_tree::{NodeId, NodeRef};
use scraper::{Html, Node, Selector};
use typst::diag::StrResult;

use crate::compile::BuildConfig;

struct Note {
    id: String,
    path: PathBuf,
    document: Html,
    transcludes: Vec<String>,
    links_out: Vec<String>,
}

pub fn process_html(build_config: &BuildConfig, html_dir: &Path) -> StrResult<()> {
    let template_path = Path::new("_template/template.html");
    let public_dir = Path::new("public");
    let output_dir = &build_config.output_directory;

    let template_html = fs::read_to_string(template_path)
        .map_err(|err| eco_format!("failed to read template {}: {err}", template_path.display()))?;

    let notes = load_notes(html_dir)?;
    let order = topo_sort_transclusions(&notes)?;

    let note_ids: HashSet<String> = notes.keys().cloned().collect();
    let mut processed_bodies = HashMap::new();
    let mut processed_heads = HashMap::new();
    let mut hide_template_ids = HashMap::new();

    for note_id in &order {
        let note = notes
            .get(note_id)
            .ok_or_else(|| eco_format!("missing note {note_id} during processing"))?;

        let body_html = render_note_body(note, &processed_bodies, &note_ids)?;
        let head_html = render_note_head(note)?;
        let hide_ids = extract_hide_template_ids(note)?;

        processed_bodies.insert(note_id.clone(), body_html);
        processed_heads.insert(note_id.clone(), head_html);
        hide_template_ids.insert(note_id.clone(), hide_ids);
    }

    let backlinks = compute_backlinks(&notes);
    let contexts = compute_contexts(&notes);

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
        let head_html = processed_heads
            .get(note_id)
            .ok_or_else(|| eco_format!("missing head html for {note_id}"))?;
        let body_html = processed_bodies
            .get(note_id)
            .ok_or_else(|| eco_format!("missing body html for {note_id}"))?;
        let backmatter_html =
            build_backmatter_html(note_id, &backlinks, &contexts, &processed_bodies)?;

        let hidden = hide_template_ids
            .get(note_id)
            .ok_or_else(|| eco_format!("missing template hide list for {note_id}"))?;
        let final_html = render_with_template(
            &template_html,
            head_html,
            body_html,
            &backmatter_html,
            hidden,
        )?;

        let output_path = output_dir.join(format!("{}.html", note.id));
        fs::write(&output_path, final_html).map_err(|err| {
            eco_format!(
                "failed to write output file {}: {err}",
                output_path.display()
            )
        })?;
    }

    Ok(())
}

fn load_notes(html_dir: &Path) -> StrResult<HashMap<String, Note>> {
    let mut notes = HashMap::new();
    let entries = fs::read_dir(html_dir).map_err(|err| {
        eco_format!(
            "failed to read html directory {}: {err}",
            html_dir.display()
        )
    })?;

    for entry in entries {
        let entry =
            entry.map_err(|err| eco_format!("failed to read html directory entry: {err}"))?;
        let path = entry.path();
        if path.extension().and_then(|ext| ext.to_str()) != Some("html") {
            continue;
        }

        let html = fs::read_to_string(&path)
            .map_err(|err| eco_format!("failed to read {}: {err}", path.display()))?;
        let document = Html::parse_document(&html);
        let id = extract_note_id(&document, &path)?;

        if notes.contains_key(&id) {
            return Err(eco_format!(
                "duplicate note id {id} found while reading {}",
                path.display()
            ));
        }

        let transcludes = collect_targets(&document, "notty-transclusion", &path)?;
        let links_out = collect_targets(&document, "notty-internal-link", &path)?;

        notes.insert(
            id.clone(),
            Note {
                id,
                path,
                document,
                transcludes,
                links_out,
            },
        );
    }

    if notes.is_empty() {
        return Err(eco_format!(
            "no html files found in directory {}",
            html_dir.display()
        ));
    }

    Ok(notes)
}

fn extract_note_id(document: &Html, path: &Path) -> StrResult<String> {
    let selector = Selector::parse("head meta")
        .map_err(|err| eco_format!("failed to parse selector head meta: {err}"))?;

    for element in document.select(&selector) {
        let name = element
            .value()
            .attr("name")
            .or_else(|| element.value().attr("property"))
            .or_else(|| element.value().attr("itemprop"));
        let Some(name) = name else {
            continue;
        };
        let name = name.to_ascii_lowercase();
        if (name == "id" || name == "identifier" || name == "notty-id")
            && let Some(content) = element.value().attr("content")
        {
            return Ok(content.to_string());
        }
    }

    Err(eco_format!(
        "missing identifier meta tag in {}",
        path.display()
    ))
}

fn extract_hide_template_ids(note: &Note) -> StrResult<HashSet<String>> {
    let selector = Selector::parse("head meta")
        .map_err(|err| eco_format!("failed to parse selector head meta: {err}"))?;
    let mut ids = HashSet::new();
    for element in note.document.select(&selector) {
        let name = match element.value().attr("name") {
            Some(value) => value.trim(),
            None => continue,
        };
        let Some(rest) = name.strip_prefix("hide:") else {
            continue;
        };
        let id = rest.trim();
        if !id.is_empty() {
            ids.insert(id.to_string());
        }
    }
    Ok(ids)
}

fn collect_targets(document: &Html, tag: &str, path: &Path) -> StrResult<Vec<String>> {
    let selector =
        Selector::parse(tag).map_err(|err| eco_format!("failed to parse selector {tag}: {err}"))?;
    let mut targets = Vec::new();
    for node in document.select(&selector) {
        let target = node.value().attr("target").ok_or_else(|| {
            eco_format!(
                "{tag} missing required attribute target in {}",
                path.display()
            )
        })?;
        targets.push(normalize_target(target));
    }
    Ok(targets)
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
    processed_bodies: &HashMap<String, String>,
    note_ids: &HashSet<String>,
) -> StrResult<String> {
    let selector = Selector::parse("body")
        .map_err(|err| eco_format!("failed to parse selector body: {err}"))?;
    let body = note
        .document
        .select(&selector)
        .next()
        .ok_or_else(|| eco_format!("missing <body> in {}", note.path.display()))?;

    let context = RenderContext {
        mode: RenderMode::Note {
            processed_bodies,
            note_ids,
        },
        hide_metadata_node: None,
        collapse_details_node: None,
        note_path: Some(&note.path),
    };

    render_children(*body, &context)
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
        hide_metadata_node: None,
        collapse_details_node: None,
        note_path: None,
    };

    render_children(*head, &context)
}

fn render_with_template(
    template_html: &str,
    head_html: &str,
    body_html: &str,
    backmatter_html: &str,
    hide_template_ids: &HashSet<String>,
) -> StrResult<String> {
    let document = Html::parse_document(template_html);
    let context = RenderContext {
        mode: RenderMode::Template {
            head_html,
            content_html: body_html,
            backmatter_html,
            hide_template_ids,
        },
        hide_metadata_node: None,
        collapse_details_node: None,
        note_path: None,
    };

    render_node(document.tree.root(), &context)
}

fn compute_backlinks(notes: &HashMap<String, Note>) -> HashMap<String, Vec<String>> {
    let mut backlinks: HashMap<String, HashSet<String>> = HashMap::new();
    for (source_id, note) in notes {
        for target in &note.links_out {
            backlinks
                .entry(target.clone())
                .or_default()
                .insert(source_id.clone());
        }
    }
    backlinks
        .into_iter()
        .map(|(key, value)| (key, value.into_iter().collect()))
        .collect()
}

fn compute_contexts(notes: &HashMap<String, Note>) -> HashMap<String, Vec<String>> {
    let mut contexts: HashMap<String, HashSet<String>> = HashMap::new();
    for (source_id, note) in notes {
        for target in &note.transcludes {
            contexts
                .entry(target.clone())
                .or_default()
                .insert(source_id.clone());
        }
    }
    contexts
        .into_iter()
        .map(|(key, value)| (key, value.into_iter().collect()))
        .collect()
}

fn build_backmatter_html(
    note_id: &str,
    backlinks: &HashMap<String, Vec<String>>,
    contexts: &HashMap<String, Vec<String>>,
    processed_bodies: &HashMap<String, String>,
) -> StrResult<String> {
    let mut sections = String::new();
    if let Some(ids) = backlinks.get(note_id) {
        let section = render_backmatter_section("Backlinks", "backlinks", ids, processed_bodies)?;
        sections.push_str(&section);
    }
    if let Some(ids) = contexts.get(note_id) {
        let section = render_backmatter_section("Contexts", "contexts", ids, processed_bodies)?;
        sections.push_str(&section);
    }
    Ok(sections)
}

fn render_backmatter_section(
    title: &str,
    class_name: &str,
    note_ids: &[String],
    processed_bodies: &HashMap<String, String>,
) -> StrResult<String> {
    if note_ids.is_empty() {
        return Ok(String::new());
    }

    let mut ids = note_ids.to_vec();
    ids.sort();

    let mut out = String::new();
    out.push_str("<section class=\"backmatter ");
    out.push_str(class_name);
    out.push_str(" hide-metadata\">");
    out.push_str("<header><h2>");
    out.push_str(title);
    out.push_str("</h2></header>");
    out.push_str("<div class=\"backmatter-items\">");

    for id in ids {
        let body_html = processed_bodies
            .get(&id)
            .ok_or_else(|| eco_format!("backmatter note {id} is missing processed html"))?;
        let fragment_html = render_fragment_with_options(body_html, true, false)?;
        out.push_str(&fragment_html);
    }

    out.push_str("</div></section>");

    Ok(out)
}

struct RenderContext<'a> {
    mode: RenderMode<'a>,
    hide_metadata_node: Option<NodeId>,
    collapse_details_node: Option<NodeId>,
    note_path: Option<&'a Path>,
}

enum RenderMode<'a> {
    Note {
        processed_bodies: &'a HashMap<String, String>,
        note_ids: &'a HashSet<String>,
    },
    Template {
        head_html: &'a str,
        content_html: &'a str,
        backmatter_html: &'a str,
        hide_template_ids: &'a HashSet<String>,
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
        RenderMode::Note {
            processed_bodies,
            note_ids,
        } => {
            if tag.eq_ignore_ascii_case("notty-internal-link") {
                let target_raw = element.attr("target").ok_or_else(|| {
                    eco_format!(
                        "notty-internal-link missing target in {}",
                        path_display(context)
                    )
                })?;
                let target = normalize_target(target_raw);
                if !note_ids.contains(&target) {
                    return Err(eco_format!(
                        "internal link target {target} referenced by {} does not exist",
                        path_display(context)
                    ));
                }
                let content = render_children(node, context)?;
                return Ok(format!("<a href=\"{target}.html\">{content}</a>"));
            }

            if tag.eq_ignore_ascii_case("notty-transclusion") {
                let target_raw = element.attr("target").ok_or_else(|| {
                    eco_format!(
                        "notty-transclusion missing target in {}",
                        path_display(context)
                    )
                })?;
                let target = normalize_target(target_raw);
                let body_html = processed_bodies.get(&target).ok_or_else(|| {
                    eco_format!(
                        "transclusion target {target} referenced by {} is not processed yet",
                        path_display(context)
                    )
                })?;
                let show_metadata = parse_bool_attr(element.attr("show-metadata"), true);
                let expanded = parse_bool_attr(element.attr("expanded"), true);
                return render_fragment_with_options(body_html, show_metadata, expanded);
            }
        }
        RenderMode::Template {
            head_html,
            content_html,
            backmatter_html,
            hide_template_ids,
        } => {
            if tag.eq_ignore_ascii_case("template")
                && let Some(id) = element.attr("id")
            {
                if hide_template_ids.contains(id) {
                    return Ok(String::new());
                }
                return render_children(node, context);
            }
            if tag.eq_ignore_ascii_case("slot")
                && let Some(name) = element.attr("name")
            {
                if name == "content" {
                    return Ok(content_html.to_string());
                }
                if name == "backmatters" {
                    return Ok(backmatter_html.to_string());
                }
            }

            if tag.eq_ignore_ascii_case("head") {
                let mut out = String::new();
                let (attrs, _) = build_attributes(element, context, node.id());
                out.push('<');
                out.push_str(tag);
                out.push_str(&attrs);
                out.push('>');
                out.push_str(&render_children(node, context)?);
                out.push_str(head_html);
                out.push_str("</");
                out.push_str(tag);
                out.push('>');
                return Ok(out);
            }
        }
        RenderMode::Fragment => {
            if tag.eq_ignore_ascii_case("notty-internal-link")
                || tag.eq_ignore_ascii_case("notty-transclusion")
            {
                return Err(eco_format!(
                    "unexpected notty element in processed fragment"
                ));
            }
        }
    }

    let (attrs, is_void) = build_attributes(element, context, node.id());

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

fn build_attributes(
    element: &scraper::node::Element,
    context: &RenderContext,
    node_id: NodeId,
) -> (String, bool) {
    let mut attrs = Vec::new();
    let mut class_value: Option<String> = None;
    let is_void = is_void_element(element.name());

    for (name, value) in element.attrs() {
        if context
            .collapse_details_node
            .is_some_and(|target| target == node_id)
            && name.eq_ignore_ascii_case("open")
        {
            continue;
        }

        if name.eq_ignore_ascii_case("class") {
            class_value = Some(value.to_string());
            continue;
        }

        attrs.push((name.to_string(), value.to_string()));
    }

    if context
        .hide_metadata_node
        .is_some_and(|target| target == node_id)
    {
        let updated = match class_value {
            Some(existing) if has_class(&existing, "hide-metadata") => existing,
            Some(existing) => format!("{existing} hide-metadata"),
            None => "hide-metadata".to_string(),
        };
        class_value = Some(updated);
    }

    if let Some(class_val) = class_value {
        attrs.push(("class".to_string(), class_val));
    }

    let mut out = String::new();
    for (name, value) in attrs {
        out.push(' ');
        out.push_str(&name);
        out.push_str("=\"");
        out.push_str(&escape_attr(&value));
        out.push('"');
    }

    (out, is_void)
}

fn render_fragment_with_options(
    html: &str,
    show_metadata: bool,
    expanded: bool,
) -> StrResult<String> {
    let fragment = Html::parse_fragment(html);
    let root = fragment.tree.root();

    let hide_metadata_node = if show_metadata {
        None
    } else {
        find_first_element(root)
    };

    let collapse_details_node = if expanded {
        None
    } else {
        find_first_element_by_tag(root, "details")
    };

    let context = RenderContext {
        mode: RenderMode::Fragment,
        hide_metadata_node,
        collapse_details_node,
        note_path: None,
    };

    render_children(root, &context)
}

fn find_first_element(root: NodeRef<Node>) -> Option<NodeId> {
    for node in root.descendants() {
        if let Some(element) = node.value().as_element() {
            let name = element.name();
            if !name.eq_ignore_ascii_case("html") && !name.eq_ignore_ascii_case("body") {
                return Some(node.id());
            }
        }
    }
    None
}

fn find_first_element_by_tag(root: NodeRef<Node>, tag: &str) -> Option<NodeId> {
    for node in root.descendants() {
        if let Some(element) = node.value().as_element()
            && element.name().eq_ignore_ascii_case(tag)
        {
            return Some(node.id());
        }
    }
    None
}

fn parse_bool_attr(value: Option<&str>, default: bool) -> bool {
    match value {
        Some(v) if v.eq_ignore_ascii_case("true") => true,
        Some(v) if v.eq_ignore_ascii_case("false") => false,
        Some(_) => default,
        None => default,
    }
}

fn normalize_target(raw: &str) -> String {
    let trimmed = raw.trim();
    let normalized = trimmed.strip_prefix("notty:").unwrap_or(trimmed).trim();
    normalized.to_string()
}

fn has_class(value: &str, class: &str) -> bool {
    value.split_whitespace().any(|item| item == class)
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
