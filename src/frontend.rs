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

    let mut compiled_any = false;
    for entry in entries {
        let entry =
            entry.map_err(|err| eco_format!("failed to read input directory entry: {err}"))?;
        let path = entry.path();
        if path.extension().and_then(|ext| ext.to_str()) != Some("typ") {
            continue;
        }
        compiled_any = true;
        compile_typst_file(build_config, &path, output_dir)?;
    }

    if !compiled_any {
        return Err(eco_format!(
            "no .typ files found in input directory {}",
            input_dir.display()
        ));
    }

    Ok(output_dir.clone())
}

fn compile_typst_file(
    build_config: &BuildConfig,
    source: &Path,
    output_dir: &Path,
) -> StrResult<()> {
    let file_stem = source
        .file_stem()
        .and_then(|stem| stem.to_str())
        .ok_or_else(|| eco_format!("invalid input filename {}", source.display()))?;
    let output_path = output_dir.join(format!("{file_stem}.html"));

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

    cmd.arg("--input").arg("notty-target=html");

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
