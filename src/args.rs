use std::fmt::{self, Display, Formatter};
use std::num::NonZeroUsize;
use std::ops::RangeInclusive;
use std::path::PathBuf;
use std::str::FromStr;

use clap::builder::{BoolishValueParser, TypedValueParser, ValueParser};
use clap::{ArgAction, Args, Parser, Subcommand, ValueEnum, ValueHint};
use clap_complete::Shell;
use serde::Deserialize;

/// The character typically used to separate path components
/// in environment variables.
const ENV_PATH_SEP: char = if cfg!(windows) { ';' } else { ':' };

/// The overall structure of the help.
#[rustfmt::skip]
const HELP_TEMPLATE: &str = "\
Weibian (wb) {version}

{usage-heading} {usage}

{all-args}{after-help}\
";

/// Adds a list of useful links after the normal help.
#[rustfmt::skip]
const AFTER_HELP: &str = color_print::cstr!("\
<s>Repository:</>                 https://github.com/hanwenguo/weibian/
");

/// The Weibian CLI.
#[derive(Debug, Clone, Parser)]
#[clap(
    name = "wb",
    version = crate::weibian_version(),
    author,
    help_template = HELP_TEMPLATE,
    after_help = AFTER_HELP,
    max_term_width = 80,
)]
pub struct CliArguments {
    /// Global arguments.
    #[clap(flatten)]
    pub global: GlobalArgs,

    /// The command to run.
    #[command(subcommand)]
    pub command: Command,
}

/// Arguments shared by all commands.
#[derive(Debug, Clone, Args)]
pub struct GlobalArgs {
    /// Path to a Weibian configuration file.
    #[arg(
        long = "config-file",
        value_name = "PATH",
        value_hint = ValueHint::FilePath,
        global = true
    )]
    pub config_file: Option<PathBuf>,
}

/// What to do.
#[derive(Debug, Clone, Subcommand)]
#[command()]
pub enum Command {
    /// Compiles Typst notes to a bundle.
    #[command(visible_alias = "c")]
    Compile(CompileCommand),

    /// Watches Typst notes and recompiles on changes.
    #[command(visible_alias = "w")]
    Watch(WatchCommand),

    /// Runs a TFP-compatible formula preview server over stdin and stdout.
    TfpServer(TfpServerCommand),

    /// Generates a shell completion script.
    Completions {
        /// Shell whose completion script should be generated.
        #[arg(value_enum)]
        shell: Shell,
    },
}

/// Runs a TFP-compatible formula preview server.
#[derive(Debug, Clone, Parser)]
pub struct TfpServerCommand {
    /// Canonical project root owned by this server process.
    #[arg(long, value_name = "PROJECT_ROOT", value_hint = ValueHint::DirPath)]
    pub root: PathBuf,
}

/// Compiles Typst notes to a bundle.
#[derive(Debug, Clone, Parser)]
pub struct CompileCommand {
    /// Arguments for compilation.
    #[clap(flatten)]
    pub args: CompileArgs,
}

/// Watches Typst notes and recompiles on changes.
#[derive(Debug, Clone, Parser)]
pub struct WatchCommand {
    /// Arguments for compilation.
    #[clap(flatten)]
    pub args: CompileArgs,

    /// Arguments for the live-reload HTTP server.
    #[clap(flatten)]
    pub server: ServerArgs,
}

/// Arguments for the live-reload HTTP server.
#[derive(Debug, Default, Clone, Parser)]
pub struct ServerArgs {
    /// Disables the built-in HTTP server for bundle export.
    #[clap(long)]
    pub no_serve: bool,

    /// Disables the injected live reload script for HTML output.
    #[clap(long)]
    pub no_reload: bool,

    /// The port where HTML is served.
    #[clap(long)]
    pub port: Option<u16>,
}

/// Arguments for compilation.
#[derive(Debug, Clone, Args)]
pub struct CompileArgs {
    /// Path to input directory (defaults to config or "typ").
    #[clap(value_hint = ValueHint::DirPath)]
    pub input: Option<PathBuf>,

    /// Path to output directory (defaults to config or "dist").
    #[clap(value_hint = ValueHint::DirPath)]
    pub output: Option<PathBuf>,

    /// Path to public assets directory, relative to input dir by default.
    #[clap(long = "public-dir", value_hint = ValueHint::DirPath)]
    pub public: Option<PathBuf>,

    /// Compiler backend to use for this compile.
    #[arg(long = "compiler", value_enum)]
    pub compiler: Option<CompilerBackendKind>,

    /// Path to the host Typst compiler executable.
    #[arg(long = "host-compiler", value_name = "PATH", value_hint = ValueHint::FilePath)]
    pub host_compiler: Option<PathBuf>,

    /// Site configuration.
    #[clap(flatten)]
    pub site: SiteArgs,

    /// Typst compile arguments that Weibian forwards.
    #[clap(flatten)]
    pub typst: TypstCompileArgs,
}

/// Which Typst compiler backend Weibian should use.
#[derive(Debug, Default, Copy, Clone, Eq, PartialEq, ValueEnum, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum CompilerBackendKind {
    /// Compile through Typst as an embedded library.
    #[default]
    Library,
    /// Compile by invoking the host `typst` binary.
    Host,
}

impl Display for CompilerBackendKind {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        self.to_possible_value()
            .expect("no values are skipped")
            .get_name()
            .fmt(f)
    }
}

impl CompilerBackendKind {
    pub fn compiled_default() -> Self {
        if cfg!(feature = "library-compiler") {
            Self::Library
        } else {
            Self::Host
        }
    }
}

/// Site configuration overrides.
#[derive(Debug, Clone, Args)]
pub struct SiteArgs {
    /// The domain of the site used for generating absolute URLs.
    #[arg(long = "site-domain", value_name = "DOMAIN")]
    pub domain: Option<String>,

    /// Root directory of the site (for example, "/notes/").
    #[arg(long = "site-root-dir", value_name = "DIR")]
    pub root_dir: Option<String>,

    /// Whether note URLs should end with a trailing slash.
    #[arg(
        long = "trailing-slash",
        value_parser = BoolishValueParser::new(),
        value_name = "BOOL"
    )]
    pub trailing_slash: Option<bool>,
}

/// Typst compile arguments that Weibian forwards transparently.
#[derive(Debug, Clone, Args)]
pub struct TypstCompileArgs {
    /// Add a string key-value pair visible through `sys.inputs`.
    #[clap(
        long = "input",
        value_name = "key=value",
        action = ArgAction::Append,
        value_parser = ValueParser::new(parse_sys_input_pair),
    )]
    pub inputs: Vec<(String, String)>,

    /// Common font arguments.
    #[clap(flatten)]
    pub font: FontArgs,

    /// Arguments related to storage of packages in the system.
    #[clap(flatten)]
    pub package: PackageArgs,

    /// The document's creation date formatted as a UNIX timestamp.
    #[clap(
        long = "creation-timestamp",
        env = "SOURCE_DATE_EPOCH",
        value_name = "UNIX_TIMESTAMP",
        value_parser = parse_creation_timestamp,
    )]
    pub creation_timestamp: Option<i64>,

    /// Whether to pretty-print produced output.
    #[arg(long = "pretty")]
    pub pretty: bool,

    /// Which pages to export. When unspecified, all pages are exported.
    #[arg(long = "pages", value_delimiter = ',')]
    pub pages: Option<Vec<Pages>>,

    /// One (or multiple comma-separated) PDF standards that Typst will enforce
    /// conformance with.
    #[arg(long = "pdf-standard", value_delimiter = ',')]
    pub pdf_standard: Vec<PdfStandard>,

    /// Disables tagged PDF output.
    #[arg(long = "no-pdf-tags")]
    pub no_pdf_tags: bool,

    /// The PPI (pixels per inch) to use for PNG export.
    #[arg(long = "ppi", default_value_t = 144.0)]
    pub ppi: f64,

    /// File path to which a Makefile with the current compilation's
    /// dependencies will be written.
    #[clap(long = "make-deps", value_name = "PATH", hide = true)]
    pub make_deps: Option<PathBuf>,

    /// File path to which a list of current compilation's dependencies will
    /// be written. Use `-` to write to stdout.
    #[clap(
        long,
        value_name = "PATH",
        value_parser = output_value_parser(),
        value_hint = ValueHint::FilePath,
    )]
    pub deps: Option<CompileOutputPath>,

    /// File format to use for dependencies.
    #[clap(long, default_value_t)]
    pub deps_format: DepsFormat,

    /// Processing arguments.
    #[clap(flatten)]
    pub process: ProcessArgs,

    /// Opens the output file with the default viewer or a specific program
    /// after compilation.
    #[arg(long = "open", value_name = "VIEWER")]
    pub open: Option<Option<String>>,

    /// Produces performance timings of the compilation process.
    #[arg(long = "timings", value_name = "OUTPUT_JSON")]
    pub timings: Option<PathBuf>,
}

impl Default for TypstCompileArgs {
    fn default() -> Self {
        Self {
            inputs: vec![],
            font: FontArgs::default(),
            package: PackageArgs::default(),
            creation_timestamp: None,
            pretty: false,
            pages: None,
            pdf_standard: vec![],
            no_pdf_tags: false,
            ppi: 144.0,
            make_deps: None,
            deps: None,
            deps_format: DepsFormat::default(),
            process: ProcessArgs::default(),
            open: None,
            timings: None,
        }
    }
}

/// Arguments for configuring the compilation process itself.
#[derive(Debug, Default, Clone, Args)]
pub struct ProcessArgs {
    /// Number of parallel jobs spawned during compilation. Defaults to number
    /// of CPUs. Setting it to 1 disables parallelism.
    #[clap(long, short)]
    pub jobs: Option<usize>,

    /// The format to emit diagnostics in.
    #[clap(long, default_value_t)]
    pub diagnostic_format: DiagnosticFormat,
}

/// Arguments related to where packages are stored in the system.
#[derive(Debug, Default, Clone, Args)]
pub struct PackageArgs {
    /// Custom path to local packages, defaults to system-dependent location.
    #[clap(long = "package-path", env = "TYPST_PACKAGE_PATH", value_name = "DIR")]
    pub package_path: Option<PathBuf>,

    /// Custom path to package cache, defaults to system-dependent location.
    #[clap(
        long = "package-cache-path",
        env = "TYPST_PACKAGE_CACHE_PATH",
        value_name = "DIR"
    )]
    pub package_cache_path: Option<PathBuf>,
}

/// Common arguments to customize available fonts.
#[derive(Debug, Default, Clone, Parser)]
pub struct FontArgs {
    /// Adds additional directories that are recursively searched for fonts.
    ///
    /// If multiple paths are specified, they are separated by the system's path
    /// separator (`:` on Unix-like systems and `;` on Windows).
    #[clap(
        long = "font-path",
        env = "TYPST_FONT_PATHS",
        value_name = "DIR",
        value_delimiter = ENV_PATH_SEP,
    )]
    pub font_paths: Vec<PathBuf>,

    /// Ensures system fonts won't be searched, unless explicitly included via
    /// `--font-path`.
    #[arg(long, env = "TYPST_IGNORE_SYSTEM_FONTS")]
    pub ignore_system_fonts: bool,

    /// Ensures fonts embedded into Typst won't be considered.
    #[arg(long, env = "TYPST_IGNORE_EMBEDDED_FONTS")]
    pub ignore_embedded_fonts: bool,
}

/// Output path for Typst flags that support stdout.
#[derive(Debug, Clone, Eq, PartialEq)]
pub enum CompileOutputPath {
    Stdout,
    Path(PathBuf),
}

impl Display for CompileOutputPath {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        match self {
            Self::Stdout => f.pad("-"),
            Self::Path(path) => path.display().fmt(f),
        }
    }
}

macro_rules! display_possible_values {
    ($ty:ty) => {
        impl Display for $ty {
            fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
                self.to_possible_value()
                    .expect("no values are skipped")
                    .get_name()
                    .fmt(f)
            }
        }
    };
}

/// File format to use for generated dependency files.
#[derive(Debug, Default, Copy, Clone, Eq, PartialEq, ValueEnum)]
pub enum DepsFormat {
    /// Encodes as JSON, failing for non-Unicode paths.
    #[default]
    Json,
    /// Separates paths with NULL bytes and can express all paths.
    Zero,
    /// Emits in Make format, omitting inexpressible paths.
    Make,
}

display_possible_values!(DepsFormat);

/// Which format to use for diagnostics.
#[derive(Debug, Default, Copy, Clone, Eq, PartialEq, Ord, PartialOrd, ValueEnum)]
pub enum DiagnosticFormat {
    #[default]
    Human,
    Short,
}

display_possible_values!(DiagnosticFormat);

/// A PDF standard that Typst can enforce conformance with.
#[derive(Debug, Copy, Clone, Eq, PartialEq, ValueEnum)]
#[allow(non_camel_case_types)]
pub enum PdfStandard {
    /// PDF 1.4.
    #[value(name = "1.4")]
    V_1_4,
    /// PDF 1.5.
    #[value(name = "1.5")]
    V_1_5,
    /// PDF 1.6.
    #[value(name = "1.6")]
    V_1_6,
    /// PDF 1.7.
    #[value(name = "1.7")]
    V_1_7,
    /// PDF 2.0.
    #[value(name = "2.0")]
    V_2_0,
    /// PDF/A-1b.
    #[value(name = "a-1b")]
    A_1b,
    /// PDF/A-1a.
    #[value(name = "a-1a")]
    A_1a,
    /// PDF/A-2b.
    #[value(name = "a-2b")]
    A_2b,
    /// PDF/A-2u.
    #[value(name = "a-2u")]
    A_2u,
    /// PDF/A-2a.
    #[value(name = "a-2a")]
    A_2a,
    /// PDF/A-3b.
    #[value(name = "a-3b")]
    A_3b,
    /// PDF/A-3u.
    #[value(name = "a-3u")]
    A_3u,
    /// PDF/A-3a.
    #[value(name = "a-3a")]
    A_3a,
    /// PDF/A-4.
    #[value(name = "a-4")]
    A_4,
    /// PDF/A-4f.
    #[value(name = "a-4f")]
    A_4f,
    /// PDF/A-4e.
    #[value(name = "a-4e")]
    A_4e,
    /// PDF/UA-1.
    #[value(name = "ua-1")]
    UA_1,
}

display_possible_values!(PdfStandard);

/// A page range forwarded to Typst.
#[derive(Debug, Clone, Eq, PartialEq)]
pub struct Pages(pub RangeInclusive<Option<NonZeroUsize>>);

impl Display for Pages {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        match (self.0.start(), self.0.end()) {
            (Some(start), Some(end)) if start == end => write!(f, "{start}"),
            (Some(start), Some(end)) => write!(f, "{start}-{end}"),
            (Some(start), None) => write!(f, "{start}-"),
            (None, Some(end)) => write!(f, "-{end}"),
            (None, None) => f.write_str("-"),
        }
    }
}

impl FromStr for Pages {
    type Err = &'static str;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        match value
            .split('-')
            .map(str::trim)
            .collect::<Vec<_>>()
            .as_slice()
        {
            [] | [""] => Err("page export range must not be empty"),
            [single_page] => {
                let page_number = parse_page_number(single_page)?;
                Ok(Self(Some(page_number)..=Some(page_number)))
            }
            ["", ""] => Err("page export range must have start or end"),
            [start, ""] => Ok(Self(Some(parse_page_number(start)?)..=None)),
            ["", end] => Ok(Self(None..=Some(parse_page_number(end)?))),
            [start, end] => {
                let start = parse_page_number(start)?;
                let end = parse_page_number(end)?;
                if start > end {
                    Err("page export range must end at a page after the start")
                } else {
                    Ok(Self(Some(start)..=Some(end)))
                }
            }
            [_, _, _, ..] => Err("page export range must have a single hyphen"),
        }
    }
}

/// Parses key/value pairs split by the first equal sign.
fn parse_sys_input_pair(raw: &str) -> Result<(String, String), String> {
    let (key, val) = raw
        .split_once('=')
        .ok_or("input must be a key and a value separated by an equal sign")?;
    let key = key.trim().to_owned();
    if key.is_empty() {
        return Err("the key was missing or empty".to_owned());
    }
    let val = val.trim().to_owned();
    Ok((key, val))
}

fn parse_creation_timestamp(raw: &str) -> Result<i64, String> {
    raw.parse()
        .map_err(|err| format!("timestamp must be decimal integer ({err})"))
}

fn parse_page_number(value: &str) -> Result<NonZeroUsize, &'static str> {
    if value == "0" {
        Err("page numbers start at one")
    } else {
        NonZeroUsize::from_str(value).map_err(|_| "not a valid page number")
    }
}

fn output_value_parser() -> impl TypedValueParser<Value = CompileOutputPath> {
    clap::builder::OsStringValueParser::new().try_map(|value| {
        if value.is_empty() {
            Err(clap::Error::new(clap::error::ErrorKind::InvalidValue))
        } else if value == "-" {
            Ok(CompileOutputPath::Stdout)
        } else {
            Ok(CompileOutputPath::Path(value.into()))
        }
    })
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use clap::{CommandFactory, Parser};
    use clap_complete::Shell;

    use super::{CliArguments, Command, CompilerBackendKind};

    #[test]
    fn tfp_server_command_requires_and_parses_root() {
        let cli = CliArguments::parse_from(["wb", "tfp-server", "--root", "/tmp/project"]);
        let Command::TfpServer(command) = cli.command else {
            panic!("tfp-server command should parse");
        };
        assert_eq!(command.root, PathBuf::from("/tmp/project"));
    }

    #[test]
    fn watch_command_parses_compile_and_server_options() {
        let cli = CliArguments::parse_from([
            "wb",
            "watch",
            "typ",
            "dist",
            "--compiler",
            "library",
            "--host-compiler",
            "/opt/typst/bin/typst",
            "--no-serve",
            "--no-reload",
            "--port",
            "4000",
        ]);

        let Command::Watch(command) = cli.command else {
            panic!("watch command should parse");
        };

        assert_eq!(command.args.input, Some(PathBuf::from("typ")));
        assert_eq!(command.args.output, Some(PathBuf::from("dist")));
        assert_eq!(command.args.compiler, Some(CompilerBackendKind::Library));
        assert_eq!(
            command.args.host_compiler,
            Some(PathBuf::from("/opt/typst/bin/typst"))
        );
        assert!(command.server.no_serve);
        assert!(command.server.no_reload);
        assert_eq!(command.server.port, Some(4000));
    }

    #[test]
    fn supported_shell_completions_name_the_wb_command() {
        for shell in [Shell::Bash, Shell::Zsh, Shell::Fish] {
            let mut command = CliArguments::command();
            let mut output = Vec::new();
            clap_complete::generate(shell, &mut command, "wb", &mut output);
            let output = String::from_utf8(output).unwrap();
            assert!(output.contains("wb"), "{shell} completion did not name wb");
        }
    }
}
