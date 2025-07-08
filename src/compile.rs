use std::fmt::{self, Display, Formatter};
use std::fs;
use std::io::Write;
use std::path::PathBuf;

use chrono::{DateTime, Datelike, Timelike, Utc};
use codespan_reporting::{
    diagnostic::{Diagnostic, Label},
    term,
};
use parking_lot::RwLock;
use typst::{
    WorldExt,
    diag::{At, Severity, SourceDiagnostic, SourceResult, StrResult, Warned, eco_format},
    foundations::{Datetime, Smart},
    html::HtmlDocument,
    layout::{Frame, PagedDocument},
};
use typst_pdf::{PdfOptions, PdfStandards, Timestamp};
use typst_syntax::{FileId, Source, Span};

use crate::{args, backend, frontend};
use crate::{
    args::{CompileArgs, CompileCommand, DiagnosticFormat, OutputFormat, PdfStandard},
    frontend::copied::{terminal, world::SystemWorld},
    set_failed,
};
// use crate::args::Output;
// use crate::args::Input;

type CodespanResult<T> = Result<T, CodespanError>;
type CodespanError = codespan_reporting::files::Error;

pub fn compile(command: &CompileCommand) -> StrResult<()> {
    let build_config = BuildConfig::new(&command.args)?;

    let mut world = SystemWorld::new(
        &command.args.input,
        &command.args.world,
        &command.args.process,
    )
    .map_err(|err| eco_format!("{err}"))?;

    match &build_config.input {
        BuildInput::Directory(_) => compile_multiple(&mut world, &build_config),
        _ => {
            if let BuildOutput::Directory(dir) = build_config.output {
                todo!()
            } else {
                // both input and output are not directories
                let config = SingleCompileConfig::new(
                    build_config.input.clone().into(),
                    build_config.output.clone().into(),
                    &build_config,
                )?;
                compile_once(&mut world, &config)
            }
        }
    }
}

#[derive(Clone)]
pub enum BuildInput {
    /// Stdin, represented by `-`.
    Stdin,
    /// A non-empty path to a file.
    File(PathBuf),
    /// A non-empty path to a directory.
    Directory(PathBuf),
}

impl Display for BuildInput {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        match self {
            BuildInput::Stdin => f.pad("stdin"),
            BuildInput::File(path) => path.display().fmt(f),
            BuildInput::Directory(path) => path.display().fmt(f),
        }
    }
}

impl From<BuildInput> for args::Input {
    fn from(input: BuildInput) -> Self {
        match input {
            BuildInput::Stdin => args::Input::Stdin,
            BuildInput::File(path) => args::Input::Path(path),
            BuildInput::Directory(path) => args::Input::Path(path),
        }
    }
}

#[derive(Clone)]
pub enum BuildOutput {
    Stdout,
    File(PathBuf),
    Directory(PathBuf),
}

impl Display for BuildOutput {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        match self {
            BuildOutput::Stdout => f.pad("stdout"),
            BuildOutput::File(path) => path.display().fmt(f),
            BuildOutput::Directory(path) => path.display().fmt(f),
        }
    }
}

impl From<BuildOutput> for args::Output {
    fn from(output: BuildOutput) -> Self {
        match output {
            BuildOutput::Stdout => args::Output::Stdout,
            BuildOutput::File(path) => args::Output::Path(path),
            BuildOutput::Directory(path) => args::Output::Path(path),
        }
    }
}

/// A preprocessed `CompileCommand`.
pub struct BuildConfig {
    pub input: BuildInput,
    pub output: BuildOutput,
    pub creation_timestamp: Option<DateTime<Utc>>,
    pub diagnostic_format: DiagnosticFormat,
    pub pdf_standards: PdfStandards,
}

impl BuildConfig {
    pub fn new(args: &CompileArgs) -> StrResult<Self> {
        let input = args.input.clone();

        let input = if let args::Input::Path(path) = input {
            if path.is_dir() {
                BuildInput::Directory(path)
            } else {
                BuildInput::File(path)
            }
        } else {
            BuildInput::Stdin
        };

        let output = args.output.clone();
        let output = if let args::Output::Path(path) = output {
            if path.is_dir() {
                BuildOutput::Directory(path)
            } else {
                BuildOutput::File(path)
            }
        } else {
            BuildOutput::Stdout
        };

        // when output is a file or stdout, input must not be a directory
        if let BuildInput::Directory(_) = input {
            match output {
                BuildOutput::Stdout => {
                    return Err(eco_format!(
                        "Cannot write to stdout when input is a directory"
                    ));
                }
                BuildOutput::File(_) => {
                    return Err(eco_format!(
                        "Cannot write to a file when input is a directory"
                    ));
                }
                _ => {}
            }
        }

        let pdf_standards = {
            let list = args
                .pdf_standard
                .iter()
                .map(|standard| match standard {
                    PdfStandard::V_1_7 => typst_pdf::PdfStandard::V_1_7,
                    PdfStandard::A_2b => typst_pdf::PdfStandard::A_2b,
                    PdfStandard::A_3b => typst_pdf::PdfStandard::A_3b,
                })
                .collect::<Vec<_>>();
            PdfStandards::new(&list)?
        };

        Ok(Self {
            input,
            output,
            creation_timestamp: args.world.creation_timestamp,
            diagnostic_format: args.process.diagnostic_format,
            pdf_standards,
        })
    }
}

/// A single compilation configuration.
pub struct SingleCompileConfig<'a> {
    pub input: args::Input,
    pub output: args::Output,
    pub build_config: &'a BuildConfig,
}

impl<'a> SingleCompileConfig<'a> {
    pub fn new(
        input: args::Input,
        output: args::Output,
        build_config: &'a BuildConfig,
    ) -> StrResult<Self> {
        Ok(Self {
            input,
            output,
            build_config,
        })
    }
}

/// Compile multiple files according to the build config.
pub fn compile_multiple(world: &mut SystemWorld, build_config: &BuildConfig) -> StrResult<()> {
    todo!()
}

/// Compile a single time.
///
/// Returns whether it compiled without errors.
pub fn compile_once(world: &mut SystemWorld, config: &SingleCompileConfig) -> StrResult<()> {
    let Warned { output, warnings } = compile_and_export(world, config);

    match output {
        // Export the PDF / PNG.
        Ok(outputs) => {
            print_diagnostics(world, &[], &warnings, config.build_config.diagnostic_format)
                .map_err(|err| eco_format!("failed to print diagnostics ({err})"))?;
        }

        // Print diagnostics.
        Err(errors) => {
            set_failed();

            print_diagnostics(
                world,
                &errors,
                &warnings,
                config.build_config.diagnostic_format,
            )
            .map_err(|err| eco_format!("failed to print diagnostics ({err})"))?;
        }
    }

    Ok(())
}

/// Compile and then export the document.
fn compile_and_export(
    world: &mut SystemWorld,
    config: &SingleCompileConfig,
) -> Warned<SourceResult<()>> {
    let Warned { output, warnings } = frontend::compile(world);
    let result = output.and_then(|document| {
        backend::compile_and_export(world, config, document) // ideally, the backend should only generate legal Typst code
    });
    Warned {
        output: result,
        warnings,
    }
    // match config.output_format {
    //     OutputFormat::Html => {
    //         let Warned { output, warnings } = typst::compile::<HtmlDocument>(world);
    //         let result = output.and_then(|document| export_html(&document, config));
    //         Warned {
    //             output: result.map(|()| vec![config.output.clone()]),
    //             warnings,
    //         }
    //     }
    //     OutputFormat::Pdf => {
    //         let Warned { output, warnings } = typst::compile::<PagedDocument>(world);
    //         let result = output.and_then(|document| export_paged(&document, config));
    //         Warned { output: result, warnings }
    //     }
    //     OutputFormat::All => {
    //         todo!();
    //     }
    // }
}

/// Caches exported files so that we can avoid re-exporting them if they haven't
/// changed.
///
/// This is done by having a list of size `files.len()` that contains the hashes
/// of the last rendered frame in each file. If a new frame is inserted, this
/// will invalidate the rest of the cache, this is deliberate as to decrease the
/// complexity and memory usage of such a cache.
pub struct ExportCache {
    /// The hashes of last compilation's frames.
    pub cache: RwLock<Vec<u128>>,
}

impl ExportCache {
    /// Creates a new export cache.
    pub fn new() -> Self {
        Self {
            cache: RwLock::new(Vec::with_capacity(32)),
        }
    }

    /// Returns true if the entry is cached and appends the new hash to the
    /// cache (for the next compilation).
    pub fn is_cached(&self, i: usize, frame: &Frame) -> bool {
        let hash = typst::utils::hash128(frame);

        let mut cache = self.cache.upgradable_read();
        if i >= cache.len() {
            cache.with_upgraded(|cache| cache.push(hash));
            return false;
        }

        cache.with_upgraded(|cache| std::mem::replace(&mut cache[i], hash) == hash)
    }
}

/// Print diagnostic messages to the terminal.
pub fn print_diagnostics(
    world: &SystemWorld,
    errors: &[SourceDiagnostic],
    warnings: &[SourceDiagnostic],
    diagnostic_format: DiagnosticFormat,
) -> Result<(), codespan_reporting::files::Error> {
    let mut config = term::Config {
        tab_width: 2,
        ..Default::default()
    };
    if diagnostic_format == DiagnosticFormat::Short {
        config.display_style = term::DisplayStyle::Short;
    }

    for diagnostic in warnings.iter().chain(errors) {
        let diag = match diagnostic.severity {
            Severity::Error => Diagnostic::error(),
            Severity::Warning => Diagnostic::warning(),
        }
        .with_message(diagnostic.message.clone())
        .with_notes(
            diagnostic
                .hints
                .iter()
                .map(|e| (eco_format!("hint: {e}")).into())
                .collect(),
        )
        .with_labels(label(world, diagnostic.span).into_iter().collect());

        term::emit(&mut terminal::out(), &config, world, &diag)?;

        // Stacktrace-like helper diagnostics.
        for point in &diagnostic.trace {
            let message = point.v.to_string();
            let help = Diagnostic::help()
                .with_message(message)
                .with_labels(label(world, point.span).into_iter().collect());

            term::emit(&mut terminal::out(), &config, world, &help)?;
        }
    }

    Ok(())
}

/// Create a label for a span.
fn label(world: &SystemWorld, span: Span) -> Option<Label<FileId>> {
    Some(Label::primary(span.id()?, world.range(span)?))
}

impl<'a> codespan_reporting::files::Files<'a> for SystemWorld {
    type FileId = FileId;
    type Name = String;
    type Source = Source;

    fn name(&'a self, id: FileId) -> CodespanResult<Self::Name> {
        let vpath = id.vpath();
        Ok(if let Some(package) = id.package() {
            format!("{package}{}", vpath.as_rooted_path().display())
        } else {
            // Try to express the path relative to the working directory.
            vpath
                .resolve(self.root())
                .and_then(|abs| pathdiff::diff_paths(abs, self.workdir()))
                .as_deref()
                .unwrap_or_else(|| vpath.as_rootless_path())
                .to_string_lossy()
                .into()
        })
    }

    fn source(&'a self, id: FileId) -> CodespanResult<Self::Source> {
        Ok(self.lookup(id))
    }

    fn line_index(&'a self, id: FileId, given: usize) -> CodespanResult<usize> {
        let source = self.lookup(id);
        source
            .byte_to_line(given)
            .ok_or_else(|| CodespanError::IndexTooLarge {
                given,
                max: source.len_bytes(),
            })
    }

    fn line_range(&'a self, id: FileId, given: usize) -> CodespanResult<std::ops::Range<usize>> {
        let source = self.lookup(id);
        source
            .line_to_range(given)
            .ok_or_else(|| CodespanError::LineTooLarge {
                given,
                max: source.len_lines(),
            })
    }

    fn column_number(&'a self, id: FileId, _: usize, given: usize) -> CodespanResult<usize> {
        let source = self.lookup(id);
        source.byte_to_column(given).ok_or_else(|| {
            let max = source.len_bytes();
            if given <= max {
                CodespanError::InvalidCharBoundary { given }
            } else {
                CodespanError::IndexTooLarge { given, max }
            }
        })
    }
}
