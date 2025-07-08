use std::{fs, io::Write};

use chrono::{Datelike, Timelike};
use typst::{diag::{eco_format, At, SourceResult, StrResult}, foundations::{Datetime, Smart}, html::HtmlDocument, layout::PagedDocument};
use typst_pdf::{PdfOptions, Timestamp};
use typst_syntax::Span;

use crate::{args, frontend::copied::world::SystemWorld};
use crate::compile::SingleCompileConfig;

pub fn compile_and_export(world: &mut SystemWorld, config: &SingleCompileConfig, result: HtmlDocument) -> SourceResult<()> {
    export_html(&result, config)
    // export_paged(&result, config)?;
    // Ok(())
}

/// Export to HTML.
fn export_html(document: &HtmlDocument, config: &SingleCompileConfig) -> SourceResult<()> {
    let html = typst_html::html(document)?;
    let result = config.output.write(html.as_bytes());

    result
        .map_err(|err| eco_format!("failed to write HTML file ({err})"))
        .at(Span::detached())
}

/// Export to a PDF.
fn export_pdf(document: &PagedDocument, config: &SingleCompileConfig) -> SourceResult<()> {
    // If the timestamp is provided through the CLI, use UTC suffix,
    // else, use the current local time and timezone.
    let timestamp = match config.build_config.creation_timestamp {
        Some(timestamp) => convert_datetime(timestamp).map(Timestamp::new_utc),
        None => {
            let local_datetime = chrono::Local::now();
            convert_datetime(local_datetime).and_then(|datetime| {
                Timestamp::new_local(
                    datetime,
                    local_datetime.offset().local_minus_utc() / 60,
                )
            })
        }
    };
    let options = PdfOptions {
        ident: Smart::Auto,
        timestamp,
        page_ranges: None,
        standards: config.build_config.pdf_standards.clone(),
    };
    let buffer = typst_pdf::pdf(document, &options)?;
    config
        .output
        .write(&buffer)
        .map_err(|err| eco_format!("failed to write PDF file ({err})"))
        .at(Span::detached())?;
    Ok(())
}

/// Convert [`chrono::DateTime`] to [`Datetime`]
fn convert_datetime<Tz: chrono::TimeZone>(
    date_time: chrono::DateTime<Tz>,
) -> Option<Datetime> {
    Datetime::from_ymd_hms(
        date_time.year(),
        date_time.month().try_into().ok()?,
        date_time.day().try_into().ok()?,
        date_time.hour().try_into().ok()?,
        date_time.minute().try_into().ok()?,
        date_time.second().try_into().ok()?,
    )
}

impl args::Output {
    fn write(&self, buffer: &[u8]) -> StrResult<()> {
        match self {
            args::Output::Stdout => std::io::stdout().write_all(buffer),
            args::Output::Path(path) => fs::write(path, buffer),
        }
        .map_err(|err| eco_format!("{err}"))
    }
}