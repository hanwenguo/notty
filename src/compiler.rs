use std::path::Path;
use std::process::Command;

use ecow::eco_format;

use crate::config::BuildConfig;
use crate::error::StrResult;

#[derive(Debug, Copy, Clone, Eq, PartialEq)]
pub enum CompileTarget {
    Html,
    Pdf,
}

impl CompileTarget {
    fn as_str(self) -> &'static str {
        match self {
            Self::Html => "html",
            Self::Pdf => "pdf",
        }
    }
}

#[derive(Debug, Copy, Clone)]
pub enum CompileOutput<'a> {
    Stdout,
    File(&'a Path),
}

#[derive(Debug)]
pub struct CompileRequest<'a> {
    pub source: &'a Path,
    pub target: CompileTarget,
    pub output: CompileOutput<'a>,
    pub additional_inputs: &'a [(&'a str, &'a str)],
}

#[derive(Debug)]
pub enum CompileArtifact {
    Stdout(Vec<u8>),
    FileWritten,
}

pub trait TypstCompiler {
    fn compile(
        &self,
        build_config: &BuildConfig,
        request: &CompileRequest<'_>,
    ) -> StrResult<CompileArtifact>;
}

#[derive(Debug, Default)]
pub struct CliTypstCompiler;

impl TypstCompiler for CliTypstCompiler {
    fn compile(
        &self,
        build_config: &BuildConfig,
        request: &CompileRequest<'_>,
    ) -> StrResult<CompileArtifact> {
        let root = build_config
            .world
            .root
            .as_ref()
            .unwrap_or(&build_config.input_directory);

        let mut cmd = Command::new("typst");
        cmd.arg("compile")
            .arg("--format")
            .arg(request.target.as_str())
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

        for (key, value) in generate_inputs_from_build_config(build_config, request.target) {
            cmd.arg("--input").arg(format!("{key}={value}"));
        }

        for (key, value) in request.additional_inputs {
            cmd.arg("--input").arg(format!("{key}={value}"));
        }

        cmd.arg(request.source);
        match request.output {
            CompileOutput::Stdout => {
                cmd.arg("-");
            }
            CompileOutput::File(path) => {
                cmd.arg(path);
            }
        }

        let output = cmd.output().map_err(|err| {
            eco_format!(
                "failed to run typst for {}: {err}",
                request.source.display()
            )
        })?;

        if !output.status.success() {
            let destination = match request.output {
                CompileOutput::Stdout => "-".to_string(),
                CompileOutput::File(path) => path.display().to_string(),
            };
            return Err(eco_format!(
                "typst compile failed for {} -> {}: {}",
                request.source.display(),
                destination,
                String::from_utf8_lossy(&output.stderr)
            ));
        }

        match request.output {
            CompileOutput::Stdout => Ok(CompileArtifact::Stdout(output.stdout)),
            CompileOutput::File(_) => Ok(CompileArtifact::FileWritten),
        }
    }
}

fn generate_inputs_from_build_config(
    build_config: &BuildConfig,
    target: CompileTarget,
) -> Vec<(String, String)> {
    let mut inputs = Vec::new();
    inputs.push((
        "wb-domain".to_string(),
        build_config
            .site
            .domain
            .as_deref()
            .unwrap_or("")
            .to_string(),
    ));
    inputs.push((
        "wb-root-dir".to_string(),
        build_config.site.root_dir.clone(),
    ));
    inputs.push((
        "wb-trailing-slash".to_string(),
        if build_config.site.trailing_slash {
            "true".to_string()
        } else {
            "false".to_string()
        },
    ));
    if let Some(bibliography_config) = &build_config.bibliography {
        inputs.push((
            "wb-bib-file".to_string(),
            bibliography_config.file.display().to_string(),
        ));
        inputs.push((
            "wb-bib-template".to_string(),
            bibliography_config.template.display().to_string(),
        ));
    }
    inputs.push(("wb-target".to_string(), target.as_str().to_string()));
    inputs
}
