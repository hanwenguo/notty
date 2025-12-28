use std::fmt::{self, Display, Formatter};
use std::path::PathBuf;

use chrono::{DateTime, Utc};
use clap::builder::{BoolishValueParser, ValueParser};
use clap::{ArgAction, Args, Parser, Subcommand, ValueEnum, ValueHint};

/// The character typically used to separate path components
/// in environment variables.
const ENV_PATH_SEP: char = if cfg!(windows) { ';' } else { ':' };

/// The overall structure of the help.
#[rustfmt::skip]
const HELP_TEMPLATE: &str = "\
Notty {version}

{usage-heading} {usage}

{all-args}{after-help}\
";

/// Adds a list of useful links after the normal help.
#[rustfmt::skip]
const AFTER_HELP: &str = color_print::cstr!("\
<s>Repository:</>                 https://github.com/hanwenguo/notty/
");

/// The Notty CLI.
#[derive(Debug, Clone, Parser)]
#[clap(
    name = "notty",
    version = crate::notty_version(),
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
    /// Path to a Notty configuration file.
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
    /// Compiles input file(s) to designated output format(s).
    #[command(visible_alias = "c")]
    Compile(CompileCommand),
    // /// Watches an input file and recompiles on changes.
    // #[command(visible_alias = "w")]
    // Watch(WatchCommand),

    // /// Opens a preview server.
    // #[command(visible_alias = "s")]
    // Serve(ServeCommand),
    // /// Initializes a new project from a template.
    // Init(InitCommand),

    // /// Self update the Notty CLI.
    // #[cfg_attr(not(feature = "self-update"), clap(hide = true))]
    // Update(UpdateCommand),
}

/// Compiles input file(s) to designated output format(s).
#[derive(Debug, Clone, Parser)]
pub struct CompileCommand {
    /// Arguments for compilation.
    #[clap(flatten)]
    pub args: CompileArgs,
}

// Compiles an input file into a supported output format.
// #[derive(Debug, Clone, Parser)]
// pub struct WatchCommand {
//     /// Arguments for compilation.
//     #[clap(flatten)]
//     pub args: CompileArgs,

//     /// Arguments for the HTTP server.
//     #[cfg(feature = "http-server")]
//     #[clap(flatten)]
//     pub server: ServerArgs,
// }

// Opens a preview server.
// #[derive(Debug, Clone, Parser)]
// pub struct ServeCommand {
//     /// Arguments for the HTTP server.
//     #[clap(flatten)]
//     pub server: ServerArgs,
// }

// Initializes a new project from a template.
// #[derive(Debug, Clone, Parser)]
// pub struct InitCommand {
//     /// The template to use, e.g. `@preview/charged-ieee`.
//     ///
//     /// You can specify the version by appending e.g. `:0.1.0`. If no version is
//     /// specified, Typst will default to the latest version.
//     ///
//     /// Supports both local and published templates.
//     pub template: String,

//     /// The project directory, defaults to the template's name.
//     pub dir: Option<String>,

//     /// Arguments related to storage of packages in the system.
//     #[clap(flatten)]
//     pub package: PackageArgs,
// }

// /// Update the CLI using a pre-compiled binary from a Typst GitHub release.
// #[derive(Debug, Clone, Parser)]
// pub struct UpdateCommand {
//     /// Which version to update to (defaults to latest).
//     pub version: Option<Version>,

//     /// Forces a downgrade to an older version (required for downgrading).
//     #[clap(long, default_value_t = false)]
//     pub force: bool,

//     /// Reverts to the version from before the last update (only possible if
//     /// `typst update` has previously ran).
//     #[clap(
//         long,
//         default_value_t = false,
//         conflicts_with = "version",
//         conflicts_with = "force"
//     )]
//     pub revert: bool,

//     /// Custom path to the backup file created on update and used by `--revert`,
//     /// defaults to system-dependent location
//     #[clap(long = "backup-path", env = "TYPST_UPDATE_BACKUP_PATH", value_name = "FILE")]
//     pub backup_path: Option<PathBuf>,
// }

/// Arguments for compilation and watching.
#[derive(Debug, Clone, Args)]
pub struct CompileArgs {
    /// Path to input directory (defaults to config or "typ").
    #[clap(value_hint = ValueHint::DirPath)]
    pub input: Option<PathBuf>,

    /// Path to intermediate HTML cache directory (defaults to config or ".notty/cache").
    #[clap(
        long = "cache-dir",
        value_hint = ValueHint::DirPath
    )]
    pub html_cache: Option<PathBuf>,

    /// Path to public assets directory (defaults to config or "public").
    #[clap(long = "public-dir", value_hint = ValueHint::DirPath)]
    pub public: Option<PathBuf>,

    /// Path to output directory (defaults to config or "dist").
    #[clap(
         value_hint = ValueHint::DirPath,
     )]
    pub output: Option<PathBuf>,

    /// Site configuration.
    #[clap(flatten)]
    pub site: SiteArgs,

    // /// The format of the output file, inferred from the extension by default.
    // #[arg(long = "format", short = 'f', default_value = "all")]
    // pub format: OutputFormat,
    /// World arguments.
    #[clap(flatten)]
    pub world: WorldArgs,

    /// One (or multiple comma-separated) PDF standards that Typst will enforce
    /// conformance with.
    #[arg(long = "pdf-standard", value_delimiter = ',')]
    pub pdf_standard: Vec<PdfStandard>,

    /// Processing arguments.
    #[clap(flatten)]
    pub process: ProcessArgs,
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

/// Arguments for the construction of a world. Shared by compile, watch, and
/// query.
#[derive(Debug, Clone, Args)]
pub struct WorldArgs {
    /// Configures the project root (for absolute paths).
    #[clap(long = "root", env = "NOTTY_ROOT", value_name = "DIR", value_hint = ValueHint::DirPath, default_value = ".")]
    pub root: Option<PathBuf>,

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

    /// The project's creation date formatted as a UNIX timestamp.
    ///
    /// For more information, see <https://reproducible-builds.org/specs/source-date-epoch/>.
    #[clap(
        long = "creation-timestamp",
        env = "SOURCE_DATE_EPOCH",
        value_name = "UNIX_TIMESTAMP",
        value_parser = parse_source_date_epoch,
    )]
    pub creation_timestamp: Option<DateTime<Utc>>,
}

/// Arguments for configuration the process of compilaton itself.
#[derive(Debug, Clone, Args)]
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
#[derive(Debug, Clone, Args)]
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
#[derive(Debug, Clone, Parser)]
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
    #[arg(long)]
    pub ignore_system_fonts: bool,
}

// Arguments for the HTTP server.
// #[cfg(feature = "http-server")]
// #[derive(Debug, Clone, Parser)]
// pub struct ServerArgs {
//     /// Disables the built-in HTTP server for HTML export.
//     // #[clap(long)]
//     // pub no_serve: bool,

//     /// Disables the injected live reload script for HTML export. The HTML that
//     /// is written to disk isn't affected either way.
//     // #[clap(long)]
//     // pub no_reload: bool,

//     /// The port where HTML is served.
//     ///
//     /// Defaults to the first free port in the range 3000-3005.
//     #[clap(long)]
//     pub port: Option<u16>,
// }

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

/// Which format to use for the generated output file.
#[derive(Debug, Copy, Clone, Eq, PartialEq, Ord, PartialOrd, ValueEnum)]
pub enum OutputFormat {
    Pdf,
    Html,
    All,
}

display_possible_values!(OutputFormat);

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
    /// PDF 1.7.
    #[value(name = "1.7")]
    V_1_7,
    /// PDF/A-2b.
    #[value(name = "a-2b")]
    A_2b,
    /// PDF/A-3b.
    #[value(name = "a-3b")]
    A_3b,
}

display_possible_values!(PdfStandard);

/// Parses key/value pairs split by the first equal sign.
///
/// This function will return an error if the argument contains no equals sign
/// or contains the key (before the equals sign) is empty.
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

/// Parses a UNIX timestamp according to <https://reproducible-builds.org/specs/source-date-epoch/>
fn parse_source_date_epoch(raw: &str) -> Result<DateTime<Utc>, String> {
    let timestamp: i64 = raw
        .parse()
        .map_err(|err| format!("timestamp must be decimal integer ({err})"))?;
    DateTime::from_timestamp(timestamp, 0).ok_or_else(|| "timestamp out of range".to_string())
}
