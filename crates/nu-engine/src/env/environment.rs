use nu_protocol::Value;
use std::ffi::OsString;

use std::fmt::Debug;

pub trait Env: Debug + Send {
    fn env(&self) -> Option<Value>;
    fn path(&self) -> Option<Value>;

    fn add_env(&mut self, key: &str, value: &str);
    fn add_path(&mut self, new_path: OsString);
}

impl Env for Box<dyn Env> {
    fn env(&self) -> Option<Value> {
        (**self).env()
    }

    fn path(&self) -> Option<Value> {
        (**self).path()
    }

    fn add_env(&mut self, key: &str, value: &str) {
        (**self).add_env(key, value);
    }

    fn add_path(&mut self, new_path: OsString) {
        (**self).add_path(new_path);
    }
}
