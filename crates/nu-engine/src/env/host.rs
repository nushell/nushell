use indexmap::IndexMap;
use nu_errors::ShellError;
use nu_source::Text;
use std::ffi::OsString;
use std::fmt::Debug;

use super::basic_host::BasicHost;

pub trait Host: Debug + Send {
    fn stdout(&mut self, out: &str);
    fn stderr(&mut self, out: &str);
    fn print_err(&mut self, err: ShellError, source: &Text);

    fn vars(&self) -> Vec<(String, String)>;
    fn env_get(&mut self, key: OsString) -> Option<OsString>;
    fn env_set(&mut self, k: OsString, v: OsString);
    fn env_rm(&mut self, k: OsString);

    fn width(&self) -> usize;
    fn height(&self) -> usize;

    fn is_external_cmd(&self, cmd_name: &str) -> bool;
}

impl Default for Box<dyn Host> {
    fn default() -> Self {
        Box::new(BasicHost)
    }
}

impl Host for Box<dyn Host> {
    fn stdout(&mut self, out: &str) {
        (**self).stdout(out)
    }

    fn stderr(&mut self, out: &str) {
        (**self).stderr(out)
    }

    fn print_err(&mut self, err: ShellError, source: &Text) {
        (**self).print_err(err, source)
    }

    fn vars(&self) -> Vec<(String, String)> {
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

    fn width(&self) -> usize {
        (**self).width()
    }

    fn height(&self) -> usize {
        (**self).height()
    }

    fn is_external_cmd(&self, name: &str) -> bool {
        (**self).is_external_cmd(name)
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

    fn print_err(&mut self, err: ShellError, source: &Text) {
        BasicHost {}.print_err(err, source);
    }

    fn vars(&self) -> Vec<(String, String)> {
        self.env_vars
            .iter()
            .map(|(k, v)| (k.clone(), v.clone()))
            .collect::<Vec<_>>()
    }

    fn env_get(&mut self, key: OsString) -> Option<OsString> {
        let key = key.into_string().expect("Couldn't convert to string.");

        self.env_vars.get(&key).map(OsString::from)
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

    fn width(&self) -> usize {
        1
    }

    fn height(&self) -> usize {
        1
    }

    fn is_external_cmd(&self, _: &str) -> bool {
        true
    }
}
