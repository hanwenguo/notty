mod frontend;
mod backend;
mod ir;
mod args;
mod compile;
mod serve;

use std::{cell::Cell, io, io::Write, process::ExitCode, sync::LazyLock};

use clap::{error::ErrorKind, Parser};
use codespan_reporting::term;
use frontend::*;
use backend::*;
use args::*;
use termcolor::WriteColor;
use typst::diag::HintedStrResult;

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
        print_error(msg.message()).expect("failed to print error");
    }

    EXIT.with(|cell| cell.get())
}

/// Execute the requested command.
fn dispatch() -> HintedStrResult<()> {
    // let mut timer = Timer::new(&ARGS);

    match &ARGS.command {
        Command::Compile(command) => crate::compile::compile(command)?,
        // Command::Watch(command) => crate::watch::watch(&mut timer, command)?,
        Command::Serve(command) => todo!(),
        // Command::Init(command) => crate::init::init(command)?,
        // Command::Query(command) => crate::query::query(command)?,
        // Command::Fonts(command) => crate::fonts::fonts(command),
        // Command::Update(command) => crate::update::update(command)?,
    }

    Ok(())
}

/// Ensure a failure exit code.
fn set_failed() {
    EXIT.with(|cell| cell.set(ExitCode::FAILURE));
}

/// Used by `args.rs`.
fn notty_version() -> &'static str {
    env!("CARGO_PKG_VERSION")
}

/// Print an application-level error (independent from a source file).
fn print_error(msg: &str) -> io::Result<()> {
    let styles = term::Styles::default();

    let mut output = crate::frontend::copied::terminal::out();
    output.set_color(&styles.header_error)?;
    write!(output, "error")?;

    output.reset()?;
    writeln!(output, ": {msg}")
}
