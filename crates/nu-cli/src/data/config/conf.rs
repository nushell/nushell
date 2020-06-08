use nu_protocol::Value;
use std::fmt::Debug;

pub trait Conf: Debug + Send {
    fn env(&self) -> Option<Value>;
    fn path(&self) -> Option<Value>;
    fn nu_env_dirs(&self) -> Option<Value>;
    fn reload(&self);
}

impl Conf for Box<dyn Conf> {
    fn env(&self) -> Option<Value> {
        (**self).env()
    }

    fn nu_env_dirs(&self) -> Option<Value> {
        (**self).nu_env_dirs()
    }

    fn path(&self) -> Option<Value> {
        (**self).path()
    }

    fn reload(&self) {
        (**self).reload();
    }
}
