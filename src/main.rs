mod args;
mod bundle;
mod compile;
mod compiler;
mod config;
mod error;
mod terminal;
mod tfp_server;
mod watch;

use std::{cell::Cell, io, io::Write, process::ExitCode, sync::LazyLock};

use args::*;
use clap::{CommandFactory, Parser};
use codespan_reporting::term;
use termcolor::WriteColor;

use crate::error::StrResult;

thread_local! {
    /// The CLI's exit code.
    static EXIT: Cell<ExitCode> = const { Cell::new(ExitCode::SUCCESS) };
}

/// The parsed command line arguments.
static ARGS: LazyLock<CliArguments> = LazyLock::new(|| {
    CliArguments::try_parse().unwrap_or_else(|error| {
        // if error.kind() == ErrorKind::DisplayHelpOnMissingArgumentOrSubcommand {
        //     crate::greet::greet();
        // }
        error.exit();
    })
});

fn main() -> ExitCode {
    let res = dispatch();

    if let Err(msg) = res {
        set_failed();
        print_error(msg.as_ref()).expect("failed to print error");
    }

    EXIT.with(|cell| cell.get())
}

/// Execute the requested command.
fn dispatch() -> StrResult<()> {
    match &ARGS.command {
        Command::Compile(command) => {
            let config = crate::config::load_config(ARGS.global.config_file.as_deref())?;
            crate::compile::compile(command, &config)?;
        }
        Command::Watch(command) => {
            let config = crate::config::load_config(ARGS.global.config_file.as_deref())?;
            crate::watch::watch(command, &config)?;
        }
        Command::TfpServer(command) => {
            crate::tfp_server::run(&command.root, ARGS.global.config_file.as_deref())?
        }
        Command::Completions { shell } => {
            let mut command = CliArguments::command();
            clap_complete::generate(*shell, &mut command, "wb", &mut io::stdout());
        }
    }

    Ok(())
}

/// Ensure a failure exit code.
fn set_failed() {
    EXIT.with(|cell| cell.set(ExitCode::FAILURE));
}

/// Used by `args.rs`.
fn weibian_version() -> &'static str {
    env!("CARGO_PKG_VERSION")
}

/// Print an application-level error (independent from a source file).
fn print_error(msg: &str) -> io::Result<()> {
    let styles = term::Styles::default();

    let mut output = terminal::out();
    output.set_color(&styles.header_error)?;
    write!(output, "error")?;

    output.reset()?;
    writeln!(output, ": {msg}")
}
