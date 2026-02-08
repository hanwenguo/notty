use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};

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

    let mut note_sources: HashMap<String, String> = HashMap::new();
    let mut notes = Vec::new();

    let sources = collect_typst_sources(build_config)?;

    if sources.is_empty() && notes.is_empty() {
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

    notes.reserve(sources.len());
    for source in &sources {
        let note = compile_source_to_html(build_config, compiler, source.as_path(), &[])?;
        register_note(
            note,
            source.display().to_string(),
            &mut note_sources,
            &mut notes,
        )?;
    }

    Ok(notes)
}

fn compile_source_to_html(
    build_config: &BuildConfig,
    compiler: &dyn TypstCompiler,
    source: &Path,
    additional_inputs: &[(&str, &str)],
) -> StrResult<HtmlNote> {
    let request = CompileRequest {
        source,
        target: CompileTarget::Html,
        output: CompileOutput::Stdout,
        additional_inputs,
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
    crate::html::parse_note_html(&html, source)
}

fn register_note(
    note: HtmlNote,
    source_description: String,
    note_sources: &mut HashMap<String, String>,
    notes: &mut Vec<HtmlNote>,
) -> StrResult<()> {
    if let Some(previous) = note_sources.get(&note.id) {
        return Err(eco_format!(
            "duplicate note id {} found while compiling {} (already used by {})",
            note.id,
            source_description,
            previous
        ));
    }
    note_sources.insert(note.id.clone(), source_description);
    notes.push(note);
    Ok(())
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
