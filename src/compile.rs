use std::path::PathBuf;

use crate::error::StrResult;

use crate::args::{CompileArgs, CompileCommand, ProcessArgs, WorldArgs};
use crate::{backend, frontend};
// use crate::args::Output;
// use crate::args::Input;

// type CodespanResult<T> = Result<T, CodespanError>;
// type CodespanError = codespan_reporting::files::Error;

pub fn compile(command: &CompileCommand) -> StrResult<()> {
    let build_config = BuildConfig::new(&command.args)?;

    let html_dir = frontend::compile_html(&build_config)?;
    backend::process_html(&build_config, &html_dir)?;

    // let mut world = SystemWorld::new(
    //     &command.args.input,
    //     &command.args.world,
    //     &command.args.process,
    // )
    // .map_err(|err| eco_format!("{err}"))?;

    // match &build_config.input {
    //     BuildInput::Directory(_) => compile_multiple(&mut world, &build_config),
    //     _ => {
    //         if let BuildOutput::Directory(dir) = build_config.output {
    //             todo!()
    //         } else {
    //             // both input and output are not directories
    //             let config = SingleCompileConfig::new(
    //                 build_config.input.clone().into(),
    //                 build_config.output.clone().into(),
    //                 &build_config,
    //             )?;
    //             compile_once(&mut world, &config)
    //         }
    //     }
    // }
    Ok(())
}

/// A preprocessed `CompileCommand`.
pub struct BuildConfig {
    pub input_directory: PathBuf,
    pub html_cache_directory: PathBuf,
    pub public_directory: PathBuf,
    pub output_directory: PathBuf,
    pub world: WorldArgs,
    pub process: ProcessArgs,
}

impl BuildConfig {
    pub fn new(args: &CompileArgs) -> StrResult<Self> {
        Ok(Self {
            input_directory: args.input.clone(),
            html_cache_directory: args.html_cache.clone(),
            public_directory: args.public.clone(),
            output_directory: args.output.clone(),
            world: args.world.clone(),
            process: args.process.clone(),
        })
    }
}

// /// Caches exported files so that we can avoid re-exporting them if they haven't
// /// changed.
// ///
// /// This is done by having a list of size `files.len()` that contains the hashes
// /// of the last rendered frame in each file. If a new frame is inserted, this
// /// will invalidate the rest of the cache, this is deliberate as to decrease the
// /// complexity and memory usage of such a cache.
// pub struct ExportCache {
//     /// The hashes of last compilation's frames.
//     pub cache: RwLock<Vec<u128>>,
// }

// impl ExportCache {
//     /// Creates a new export cache.
//     pub fn new() -> Self {
//         Self {
//             cache: RwLock::new(Vec::with_capacity(32)),
//         }
//     }

//     /// Returns true if the entry is cached and appends the new hash to the
//     /// cache (for the next compilation).
//     pub fn is_cached(&self, i: usize, frame: &Frame) -> bool {
//         let hash = typst::utils::hash128(frame);

//         let mut cache = self.cache.upgradable_read();
//         if i >= cache.len() {
//             cache.with_upgraded(|cache| cache.push(hash));
//             return false;
//         }

//         cache.with_upgraded(|cache| std::mem::replace(&mut cache[i], hash) == hash)
//     }
// }

// /// Print diagnostic messages to the terminal.
// pub fn print_diagnostics(
//     world: &SystemWorld,
//     errors: &[SourceDiagnostic],
//     warnings: &[SourceDiagnostic],
//     diagnostic_format: DiagnosticFormat,
// ) -> Result<(), codespan_reporting::files::Error> {
//     let mut config = term::Config {
//         tab_width: 2,
//         ..Default::default()
//     };
//     if diagnostic_format == DiagnosticFormat::Short {
//         config.display_style = term::DisplayStyle::Short;
//     }

//     for diagnostic in warnings.iter().chain(errors) {
//         let diag = match diagnostic.severity {
//             Severity::Error => Diagnostic::error(),
//             Severity::Warning => Diagnostic::warning(),
//         }
//         .with_message(diagnostic.message.clone())
//         .with_notes(
//             diagnostic
//                 .hints
//                 .iter()
//                 .map(|e| (eco_format!("hint: {e}")).into())
//                 .collect(),
//         )
//         .with_labels(label(world, diagnostic.span).into_iter().collect());

//         term::emit(&mut terminal::out(), &config, world, &diag)?;

//         // Stacktrace-like helper diagnostics.
//         for point in &diagnostic.trace {
//             let message = point.v.to_string();
//             let help = Diagnostic::help()
//                 .with_message(message)
//                 .with_labels(label(world, point.span).into_iter().collect());

//             term::emit(&mut terminal::out(), &config, world, &help)?;
//         }
//     }

//     Ok(())
// }

// /// Create a label for a span.
// fn label(world: &SystemWorld, span: Span) -> Option<Label<FileId>> {
//     Some(Label::primary(span.id()?, world.range(span)?))
// }

// impl<'a> codespan_reporting::files::Files<'a> for SystemWorld {
//     type FileId = FileId;
//     type Name = String;
//     type Source = Source;

//     fn name(&'a self, id: FileId) -> CodespanResult<Self::Name> {
//         let vpath = id.vpath();
//         Ok(if let Some(package) = id.package() {
//             format!("{package}{}", vpath.as_rooted_path().display())
//         } else {
//             // Try to express the path relative to the working directory.
//             vpath
//                 .resolve(self.root())
//                 .and_then(|abs| pathdiff::diff_paths(abs, self.workdir()))
//                 .as_deref()
//                 .unwrap_or_else(|| vpath.as_rootless_path())
//                 .to_string_lossy()
//                 .into()
//         })
//     }

//     fn source(&'a self, id: FileId) -> CodespanResult<Self::Source> {
//         Ok(self.lookup(id))
//     }

//     fn line_index(&'a self, id: FileId, given: usize) -> CodespanResult<usize> {
//         let source = self.lookup(id);
//         source
//             .byte_to_line(given)
//             .ok_or_else(|| CodespanError::IndexTooLarge {
//                 given,
//                 max: source.len_bytes(),
//             })
//     }

//     fn line_range(&'a self, id: FileId, given: usize) -> CodespanResult<std::ops::Range<usize>> {
//         let source = self.lookup(id);
//         source
//             .line_to_range(given)
//             .ok_or_else(|| CodespanError::LineTooLarge {
//                 given,
//                 max: source.len_lines(),
//             })
//     }

//     fn column_number(&'a self, id: FileId, _: usize, given: usize) -> CodespanResult<usize> {
//         let source = self.lookup(id);
//         source.byte_to_column(given).ok_or_else(|| {
//             let max = source.len_bytes();
//             if given <= max {
//                 CodespanError::InvalidCharBoundary { given }
//             } else {
//                 CodespanError::IndexTooLarge { given, max }
//             }
//         })
//     }
// }
