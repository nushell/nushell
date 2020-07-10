use crate::prelude::*;
#[cfg(test)]
use indexmap::IndexMap;
// use nu_errors::ShellError;
use std::ffi::OsString;
use std::fmt::Debug;

pub trait Host: Debug + Send {
    fn out_termcolor(&self) -> termcolor::StandardStream;
    fn err_termcolor(&self) -> termcolor::StandardStream;

    fn stdout(&mut self, out: &str);
    fn stderr(&mut self, out: &str);

    fn vars(&mut self) -> Vec<(String, String)>;
    fn env_get(&mut self, key: OsString) -> Option<OsString>;
    fn env_set(&mut self, k: OsString, v: OsString);
    fn env_rm(&mut self, k: OsString);

    fn width(&self) -> usize;
}

impl Host for Box<dyn Host> {
    fn stdout(&mut self, out: &str) {
        (**self).stdout(out)
    }

    fn stderr(&mut self, out: &str) {
        (**self).stderr(out)
    }

    fn vars(&mut self) -> Vec<(String, String)> {
        (**self).vars()
    }

    fn env_get(&mut self, key: OsString) -> Option<OsString> {
        (**self).env_get(key)
    }

    fn env_set(&mut self, key: OsString, value: OsString) {
        (**self).env_set(key, value);
    }

    fn env_rm(&mut self, key: OsString) {
        (**self).env_rm(key)
    }

    fn out_termcolor(&self) -> termcolor::StandardStream {
        (**self).out_termcolor()
    }

    fn err_termcolor(&self) -> termcolor::StandardStream {
        (**self).err_termcolor()
    }

    fn width(&self) -> usize {
        (**self).width()
    }
}

#[derive(Debug)]
pub struct BasicHost;

impl Host for BasicHost {
    fn stdout(&mut self, out: &str) {
        match out {
            "\n" => outln!(""),
            other => outln!("{}", other),
        }
    }

    fn stderr(&mut self, out: &str) {
        match out {
            "\n" => errln!(""),
            other => errln!("{}", other),
        }
    }

    fn vars(&mut self) -> Vec<(String, String)> {
        #[cfg(not(target_arch = "wasm32"))]
        {
            std::env::vars().collect::<Vec<_>>()
        }

        #[cfg(target_arch = "wasm32")]
        {
            vec![]
        }
    }

    fn env_get(&mut self, key: OsString) -> Option<OsString> {
        #[cfg(not(target_arch = "wasm32"))]
        {
            std::env::var_os(key)
        }
        #[cfg(target_arch = "wasm32")]
        {
            None
        }
    }

    fn env_set(&mut self, key: OsString, value: OsString) {
        #[cfg(not(target_arch = "wasm32"))]
        {
            std::env::set_var(key, value);
        }
    }

    fn env_rm(&mut self, key: OsString) {
        #[cfg(not(target_arch = "wasm32"))]
        {
            std::env::remove_var(key);
        }
    }

    fn out_termcolor(&self) -> termcolor::StandardStream {
        termcolor::StandardStream::stdout(termcolor::ColorChoice::Auto)
    }

    fn err_termcolor(&self) -> termcolor::StandardStream {
        termcolor::StandardStream::stderr(termcolor::ColorChoice::Auto)
    }

    fn width(&self) -> usize {
        let (mut term_width, _) = term_size::dimensions().unwrap_or_else(|| (20, 20));
        term_width -= 1;
        std::cmp::max(term_width, 20)
    }
}

#[cfg(test)]
#[derive(Debug)]
pub struct FakeHost {
    line_written: String,
    env_vars: IndexMap<String, String>,
}

#[cfg(test)]
impl FakeHost {
    pub fn new() -> FakeHost {
        FakeHost {
            line_written: String::from(""),
            env_vars: IndexMap::default(),
        }
    }
}

#[cfg(test)]
impl Host for FakeHost {
    fn stdout(&mut self, out: &str) {
        self.line_written = out.to_string();
    }

    fn stderr(&mut self, out: &str) {
        self.line_written = out.to_string();
    }

    fn vars(&mut self) -> Vec<(String, String)> {
        self.env_vars
            .iter()
            .map(|(k, v)| (k.clone(), v.clone()))
            .collect::<Vec<_>>()
    }

    fn env_get(&mut self, key: OsString) -> Option<OsString> {
        let key = key.into_string().expect("Couldn't convert to string.");

        match self.env_vars.get(&key) {
            Some(env) => Some(OsString::from(env)),
            None => None,
        }
    }

    fn env_set(&mut self, key: OsString, value: OsString) {
        self.env_vars.insert(
            key.into_string().expect("Couldn't convert to string."),
            value.into_string().expect("Couldn't convert to string."),
        );
    }

    fn env_rm(&mut self, key: OsString) {
        self.env_vars
            .shift_remove(&key.into_string().expect("Couldn't convert to string."));
    }

    fn out_termcolor(&self) -> termcolor::StandardStream {
        termcolor::StandardStream::stdout(termcolor::ColorChoice::Auto)
    }

    fn err_termcolor(&self) -> termcolor::StandardStream {
        termcolor::StandardStream::stderr(termcolor::ColorChoice::Auto)
    }

    fn width(&self) -> usize {
        1
    }
}

// pub(crate) fn handle_unexpected<T>(
//     host: &mut dyn Host,
//     func: impl FnOnce(&mut dyn Host) -> Result<T, ShellError>,
// ) {
//     let result = func(host);

//     if let Err(err) = result {
//         host.stderr(&format!("Something unexpected happened:\n{:?}", err));
//     }
// }
