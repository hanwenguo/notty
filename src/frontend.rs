use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;

use crate::error::StrResult;
use ecow::eco_format;

use crate::compiler::{
    CompileArtifact, CompileOutput, CompileRequest, CompileTarget, TypstCompiler,
};
use crate::config::BuildConfig;
use crate::html::HtmlNote;

pub fn compile_html(
    build_config: &BuildConfig,
    compiler: &dyn TypstCompiler,
) -> StrResult<Vec<HtmlNote>> {
    let input_dir = &build_config.input_directory;

    let sources = collect_typst_sources(build_config)?;

    if sources.is_empty() {
        if build_config.input_filters.has_filters() {
            return Err(eco_format!(
                "no .typ files matched input include/exclude patterns in input directory {}",
                input_dir.display()
            ));
        }
        return Err(eco_format!(
            "no .typ files found in input directory {}",
            input_dir.display()
        ));
    }

    let mut note_sources: HashMap<String, PathBuf> = HashMap::new();
    let mut notes = Vec::with_capacity(sources.len());

    for source in &sources {
        let request = CompileRequest {
            source: source.as_path(),
            target: CompileTarget::Html,
            output: CompileOutput::Stdout,
            additional_inputs: &[],
        };
        let html = match compiler.compile(build_config, &request)? {
            CompileArtifact::Stdout(stdout) => String::from_utf8(stdout).map_err(|err| {
                eco_format!(
                    "typst output for {} is not valid UTF-8: {err}",
                    source.display()
                )
            })?,
            CompileArtifact::FileWritten => {
                return Err(eco_format!(
                    "typst compiler returned file output for html compilation of {}",
                    source.display()
                ));
            }
        };
        let note = crate::html::parse_note_html(&html, source)?;
        if let Some(previous) = note_sources.get(&note.id) {
            return Err(eco_format!(
                "duplicate note id {} found while compiling {} (already used by {})",
                note.id,
                source.display(),
                previous.display()
            ));
        }
        note_sources.insert(note.id.clone(), source.clone());
        notes.push(note);
    }

    Ok(notes)
}

fn collect_typst_sources(build_config: &BuildConfig) -> StrResult<Vec<PathBuf>> {
    let input_dir = &build_config.input_directory;
    let mut sources = Vec::new();
    let mut stack = vec![input_dir.clone()];

    while let Some(dir) = stack.pop() {
        let entries = fs::read_dir(&dir).map_err(|err| {
            eco_format!("failed to read input directory {}: {err}", dir.display())
        })?;

        for entry in entries {
            let entry =
                entry.map_err(|err| eco_format!("failed to read input directory entry: {err}"))?;
            let path = entry.path();
            let file_type = entry.file_type().map_err(|err| {
                eco_format!("failed to read file type for {}: {err}", path.display())
            })?;

            if file_type.is_dir() {
                stack.push(path);
                continue;
            }

            if !file_type.is_file() {
                continue;
            }

            if path.extension().and_then(|ext| ext.to_str()) != Some("typ") {
                continue;
            }

            let relative = path.strip_prefix(input_dir).unwrap_or(&path);
            if !build_config.input_filters.allows(relative) {
                continue;
            }

            sources.push(path);
        }
    }

    Ok(sources)
}
