use crate::prelude::*;

pub trait Host {
    fn out_terminal(&self) -> Box<term::StdoutTerminal>;
    fn err_terminal(&self) -> Box<term::StderrTerminal>;

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
}

crate struct BasicHost;

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
