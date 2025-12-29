use std::collections::HashSet;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

use crate::error::StrResult;
use ecow::eco_format;

use crate::config::BuildConfig;

pub fn compile_html(build_config: &BuildConfig) -> StrResult<PathBuf> {
    let input_dir = &build_config.input_directory;
    let output_dir = &build_config.html_cache_directory;

    fs::create_dir_all(output_dir).map_err(|err| {
        eco_format!(
            "failed to create html cache directory {}: {err}",
            output_dir.display()
        )
    })?;

    let entries = fs::read_dir(input_dir).map_err(|err| {
        eco_format!(
            "failed to read input directory {}: {err}",
            input_dir.display()
        )
    })?;

    let mut sources = Vec::new();
    for entry in entries {
        let entry =
            entry.map_err(|err| eco_format!("failed to read input directory entry: {err}"))?;
        let path = entry.path();
        if path.extension().and_then(|ext| ext.to_str()) != Some("typ") {
            continue;
        }
        let relative = path.strip_prefix(input_dir).unwrap_or(&path);
        if !build_config.input_filters.allows(relative) {
            continue;
        }
        sources.push(path);
    }

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

    for source in &sources {
        compile_typst_file(build_config, source, output_dir)?;
    }

    clean_html_cache(output_dir, &sources)?;

    Ok(output_dir.clone())
}

fn compile_typst_file(
    build_config: &BuildConfig,
    source: &Path,
    output_dir: &Path,
) -> StrResult<()> {
    let output_path = html_output_path(source, output_dir)?;

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

    cmd.arg("--input").arg("wb-target=html");

    cmd.arg(source).arg(&output_path);

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

    Ok(())
}

fn html_output_path(source: &Path, output_dir: &Path) -> StrResult<PathBuf> {
    let file_stem = source
        .file_stem()
        .and_then(|stem| stem.to_str())
        .ok_or_else(|| eco_format!("invalid input filename {}", source.display()))?;
    Ok(output_dir.join(format!("{file_stem}.html")))
}

fn clean_html_cache(output_dir: &Path, sources: &[PathBuf]) -> StrResult<()> {
    let mut expected = HashSet::new();
    for source in sources {
        let output_path = html_output_path(source, output_dir)?;
        if let Some(name) = output_path.file_name().and_then(|name| name.to_str()) {
            expected.insert(name.to_string());
        }
    }

    let entries = fs::read_dir(output_dir).map_err(|err| {
        eco_format!(
            "failed to read html cache directory {}: {err}",
            output_dir.display()
        )
    })?;

    for entry in entries {
        let entry = entry.map_err(|err| eco_format!("failed to read html cache entry: {err}"))?;
        let path = entry.path();
        if path.extension().and_then(|ext| ext.to_str()) != Some("html") {
            continue;
        }
        let Some(name) = path.file_name().and_then(|name| name.to_str()) else {
            continue;
        };
        if !expected.contains(name) {
            fs::remove_file(&path).map_err(|err| {
                eco_format!(
                    "failed to remove stale html cache file {}: {err}",
                    path.display()
                )
            })?;
        }
    }

    Ok(())
}
