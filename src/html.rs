use std::path::Path;

use ecow::eco_format;
use scraper::{Html, Selector};

use crate::error::StrResult;

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
