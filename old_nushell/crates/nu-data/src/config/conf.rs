use nu_errors::ShellError;
use nu_protocol::Value;
use std::{fmt::Debug, path::PathBuf};

pub trait Conf: Debug + Send {
    fn is_modified(&self) -> Result<bool, Box<dyn std::error::Error>>;
    fn var(&self, key: &str) -> Option<Value>;
    fn env(&self) -> Option<Value>;
    fn path(&self) -> Result<Option<Vec<PathBuf>>, ShellError>;
    fn clone_box(&self) -> Box<dyn Conf>;
    fn reload(&mut self);
}

impl Conf for Box<dyn Conf> {
    fn is_modified(&self) -> Result<bool, Box<dyn std::error::Error>> {
        (**self).is_modified()
    }

    fn var(&self, key: &str) -> Option<Value> {
        (**self).var(key)
    }

    fn env(&self) -> Option<Value> {
        (**self).env()
    }

    fn reload(&mut self) {
        (**self).reload();
    }

    fn clone_box(&self) -> Box<dyn Conf> {
        (**self).clone_box()
    }

    fn path(&self) -> Result<Option<Vec<PathBuf>>, ShellError> {
        (**self).path()
    }
}
