use indexmap::IndexMap;
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
    fn height(&self) -> usize;
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

    fn height(&self) -> usize {
        (**self).height()
    }
}

#[derive(Debug)]
pub struct FakeHost {
    line_written: String,
    env_vars: IndexMap<String, String>,
}

impl FakeHost {
    pub fn new() -> FakeHost {
        FakeHost {
            line_written: String::from(""),
            env_vars: IndexMap::default(),
        }
    }
}

impl Default for FakeHost {
    fn default() -> Self {
        FakeHost::new()
    }
}

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

    fn height(&self) -> usize {
        1
    }
}
