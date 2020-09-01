use nu_protocol::Value;
use std::fmt::Debug;

pub trait Conf: Debug + Send {
    fn env(&self) -> Option<Value>;
    fn path(&self) -> Option<Value>;
    fn reload(&self);
    fn clone_box(&self) -> Box<dyn Conf>;
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

    fn clone_box(&self) -> Box<dyn Conf> {
        (**self).clone_box()
    }
}
