mod args;
mod backend;
mod config;
mod error;
mod frontend;
mod html;
mod terminal;
// mod backend;
mod compile;
// mod frontend;
// mod ir;

use std::{cell::Cell, io, io::Write, process::ExitCode, sync::LazyLock};

use args::*;
// use backend::*;
use clap::Parser;
use codespan_reporting::term;
// use frontend::*;
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
    // let mut timer = Timer::new(&ARGS);
    let config = crate::config::load_config(ARGS.global.config_file.as_deref())?;

    match &ARGS.command {
        Command::Compile(command) => crate::compile::compile(command, &config)?,
        // Command::Watch(command) => crate::watch::watch(&mut timer, command)?,
        // Command::Serve(command) => todo!(),
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
