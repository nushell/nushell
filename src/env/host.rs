use crate::prelude::*;
use language_reporting::termcolor;
use std::fmt::Debug;

pub trait Host: Debug + Send {
    fn out_terminal(&self) -> Box<term::StdoutTerminal>;
    fn err_terminal(&self) -> Box<term::StderrTerminal>;

    fn out_termcolor(&self) -> termcolor::StandardStream;
    fn err_termcolor(&self) -> termcolor::StandardStream;

    fn stdout(&mut self, out: &str);
    fn stderr(&mut self, out: &str);
}

impl Host for Box<dyn Host> {
    fn out_terminal(&self) -> Box<term::StdoutTerminal> {
        (**self).out_terminal()
    }

    fn err_terminal(&self) -> Box<term::StderrTerminal> {
        (**self).err_terminal()
    }

    fn stdout(&mut self, out: &str) {
        (**self).stdout(out)
    }

    fn stderr(&mut self, out: &str) {
        (**self).stderr(out)
    }

    fn out_termcolor(&self) -> termcolor::StandardStream {
        (**self).out_termcolor()
    }

    fn err_termcolor(&self) -> termcolor::StandardStream {
        (**self).err_termcolor()
    }
}

#[derive(Debug)]
pub struct BasicHost;

impl Host for BasicHost {
    fn out_terminal(&self) -> Box<term::StdoutTerminal> {
        term::stdout().unwrap()
    }

    fn err_terminal(&self) -> Box<term::StderrTerminal> {
        term::stderr().unwrap()
    }

    fn stdout(&mut self, out: &str) {
        match out {
            "\n" => println!(""),
            other => println!("{}", other),
        }
    }

    fn stderr(&mut self, out: &str) {
        match out {
            "\n" => eprintln!(""),
            other => eprintln!("{}", other),
        }
    }

    fn out_termcolor(&self) -> termcolor::StandardStream {
        termcolor::StandardStream::stdout(termcolor::ColorChoice::Auto)
    }
    fn err_termcolor(&self) -> termcolor::StandardStream {
        termcolor::StandardStream::stderr(termcolor::ColorChoice::Auto)
    }
}

crate fn handle_unexpected<T>(
    host: &mut dyn Host,
    func: impl FnOnce(&mut dyn Host) -> Result<T, ShellError>,
) {
    let result = func(host);

    if let Err(err) = result {
        host.stderr(&format!("Something unexpected happened:\n{:?}", err));
    }
}
