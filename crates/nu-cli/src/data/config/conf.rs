use nu_protocol::Value;
use std::fmt::Debug;

pub trait Conf: Debug + Send {
    fn env(&self) -> Option<Value>;
    fn path(&self) -> Option<Value>;
    fn reload(&self);
}

impl Conf for Box<dyn Conf> {
    fn env(&self) -> Option<Value> {
        (**self).env()
    }

    fn path(&self) -> Option<Value> {
        (**self).path()
    }

    fn reload(&self) {
        (**self).reload();
    }
}
