use std::collections::HashMap;
use std::path::{Path, PathBuf};

use ecow::eco_format;
use ego_tree::{NodeId, NodeRef};
use html5ever::{LocalName, Namespace, QualName};
use scraper::{Html, Node, Selector};

use crate::error::StrResult;

pub struct HtmlNote {
    pub id: String,
    pub source_path: PathBuf,
    pub document: Html,
}

pub fn parse_note_html(html: &str, source_path: &Path) -> StrResult<HtmlNote> {
    let document = Html::parse_document(html);
    let id = extract_note_id(&document, source_path)?;
    Ok(HtmlNote {
        id,
        source_path: source_path.to_path_buf(),
        document,
    })
}

pub fn extract_note_id(document: &Html, path: &Path) -> StrResult<String> {
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
        if (name == "id" || name == "identifier" || name == "wb-id")
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

pub fn extract_metadata(document: &Html) -> StrResult<HashMap<String, String>> {
    let selector = Selector::parse("head meta")
        .map_err(|err| eco_format!("failed to parse selector head meta: {err}"))?;
    let mut metadata = HashMap::new();
    for element in document.select(&selector) {
        let Some(name) = element.value().attr("name") else {
            continue;
        };
        let Some(content) = element.value().attr("content") else {
            continue;
        };
        let key = name.trim().to_ascii_lowercase();
        if key.is_empty() {
            continue;
        }
        metadata.insert(key, content.to_string());
    }
    Ok(metadata)
}

pub fn extract_note_title(
    document: &Html,
    metadata: &HashMap<String, String>,
) -> StrResult<Option<String>> {
    if let Some(title) = metadata.get("title") {
        return Ok(Some(title.clone()));
    }

    let selector = Selector::parse("head title")
        .map_err(|err| eco_format!("failed to parse selector head title: {err}"))?;
    for element in document.select(&selector) {
        let text = element.text().collect::<String>().trim().to_string();
        if !text.is_empty() {
            return Ok(Some(text));
        }
    }

    Ok(None)
}

pub fn collect_targets(document: &Html, tag: &str, path: &Path) -> StrResult<Vec<String>> {
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

pub fn parse_bool_attr(value: Option<&str>, default: bool) -> bool {
    match value {
        Some(v) if v.eq_ignore_ascii_case("true") => true,
        Some(v) if v.eq_ignore_ascii_case("false") => false,
        Some(_) => default,
        None => default,
    }
}

pub fn normalize_target(raw: &str) -> String {
    let trimmed = raw.trim();
    let normalized = trimmed.strip_prefix("wb:").unwrap_or(trimmed).trim();
    normalized.to_string()
}

#[allow(dead_code)]
pub fn find_first_element(root: NodeRef<Node>) -> Option<NodeId> {
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

#[allow(dead_code)]
pub fn find_first_element_by_tag(root: NodeRef<Node>, tag: &str) -> Option<NodeId> {
    for node in root.descendants() {
        if let Some(element) = node.value().as_element()
            && element.name().eq_ignore_ascii_case(tag)
        {
            return Some(node.id());
        }
    }
    None
}

pub fn set_attr(element: &mut scraper::node::Element, name: &str, value: &str) {
    let existing_key = element
        .attrs
        .keys()
        .find(|key| key.local.as_ref() == name)
        .cloned();
    let key = existing_key
        .unwrap_or_else(|| QualName::new(None, Namespace::from(""), LocalName::from(name)));
    element.attrs.insert(key, value.to_string().into());
}

pub fn add_class_to_element(element: &mut scraper::node::Element, class: &str) {
    let mut class_value = element
        .attrs
        .iter()
        .find(|(key, _)| key.local.as_ref() == "class")
        .map(|(_, value)| value.to_string());
    add_class(&mut class_value, class);
    if let Some(value) = class_value {
        set_attr(element, "class", &value);
    }
}

pub fn has_class(value: &str, class: &str) -> bool {
    value.split_whitespace().any(|item| item == class)
}

pub fn add_class(class_value: &mut Option<String>, class: &str) {
    let updated = match class_value.take() {
        Some(existing) if has_class(&existing, class) => existing,
        Some(existing) => format!("{existing} {class}"),
        None => class.to_string(),
    };
    *class_value = Some(updated);
}
