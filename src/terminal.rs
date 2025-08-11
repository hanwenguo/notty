use std::io::{self, Write};

use codespan_reporting::term::termcolor;
use termcolor::{ColorChoice, WriteColor};
use typst::utils::singleton;

// use crate::ARGS;

/// Returns a handle to the optionally colored terminal output.
pub fn out() -> TermOut {
    TermOut {
        inner: singleton!(TermOutInner, TermOutInner::new()),
    }
}

/// The stuff that has to be shared between instances of [`TermOut`].
struct TermOutInner {
    stream: termcolor::StandardStream,
}

impl TermOutInner {
    fn new() -> Self {
        // let color_choice = match ARGS.color {
        //     clap::ColorChoice::Auto if std::io::stderr().is_terminal() => {
        //         ColorChoice::Auto
        //     }
        //     clap::ColorChoice::Always => ColorChoice::Always,
        //     _ => ColorChoice::Never,
        // };

        // let stream = termcolor::StandardStream::stderr(color_choice);
        let stream = termcolor::StandardStream::stderr(ColorChoice::Auto);
        TermOutInner { stream }
    }
}

/// A utility that allows users to write colored terminal output.
/// If colors are not supported by the terminal, they are disabled.
/// This type also allows for deletion of previously written lines.
#[derive(Clone)]
pub struct TermOut {
    inner: &'static TermOutInner,
}

impl TermOut {
    // Additional terminal helpers can be added here when needed.
}

impl Write for TermOut {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        self.inner.stream.lock().write(buf)
    }

    fn flush(&mut self) -> io::Result<()> {
        self.inner.stream.lock().flush()
    }
}

impl WriteColor for TermOut {
    fn supports_color(&self) -> bool {
        self.inner.stream.supports_color()
    }

    fn set_color(&mut self, spec: &termcolor::ColorSpec) -> io::Result<()> {
        self.inner.stream.lock().set_color(spec)
    }

    fn reset(&mut self) -> io::Result<()> {
        self.inner.stream.lock().reset()
    }
}
