use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

use crate::error::StrResult;
use ecow::eco_format;

use crate::config::BuildConfig;
use crate::html::HtmlNote;

pub fn compile_html(build_config: &BuildConfig) -> StrResult<Vec<HtmlNote>> {
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
        let html = compile_typst_file(build_config, source)?;
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

fn generate_inputs_from_build_config(build_config: &BuildConfig) -> Vec<String> {
    let mut inputs = Vec::new();
    inputs.push(format!(
        "wb-domain={}",
        build_config.site.domain.as_deref().unwrap_or("")
    ));
    inputs.push(format!("wb-root-dir={}", build_config.site.root_dir));
    inputs.push(format!(
        "wb-trailing-slash={}",
        if build_config.site.trailing_slash {
            "true"
        } else {
            "false"
        }
    ));
    inputs.push("wb-target=html".to_string());
    inputs
}

fn compile_typst_file(build_config: &BuildConfig, source: &Path) -> StrResult<String> {
    let root = build_config
        .world
        .root
        .as_ref()
        .unwrap_or(&build_config.input_directory);

    let mut cmd = Command::new("typst");
    cmd.arg("compile")
        .arg("--format")
        .arg("html")
        .arg("--features")
        .arg("html")
        .arg("--root")
        .arg(root);

    if let Some(jobs) = build_config.process.jobs {
        cmd.arg("--jobs").arg(jobs.to_string());
    }

    for (key, value) in &build_config.world.inputs {
        cmd.arg("--input").arg(format!("{key}={value}"));
    }

    for font_path in &build_config.world.font.font_paths {
        cmd.arg("--font-path").arg(font_path);
    }

    if build_config.world.font.ignore_system_fonts {
        cmd.arg("--ignore-system-fonts");
    }

    if let Some(path) = &build_config.world.package.package_path {
        cmd.arg("--package-path").arg(path);
    }

    if let Some(path) = &build_config.world.package.package_cache_path {
        cmd.arg("--package-cache-path").arg(path);
    }

    for input in generate_inputs_from_build_config(build_config) {
        cmd.arg("--input").arg(input);
    }

    cmd.arg(source).arg("-");

    let output = cmd
        .output()
        .map_err(|err| eco_format!("failed to run typst for {}: {err}", source.display()))?;

    if !output.status.success() {
        return Err(eco_format!(
            "typst compile failed for {}: {}",
            source.display(),
            String::from_utf8_lossy(&output.stderr)
        ));
    }

    String::from_utf8(output.stdout).map_err(|err| {
        eco_format!(
            "typst output for {} is not valid UTF-8: {err}",
            source.display()
        )
    })
}
