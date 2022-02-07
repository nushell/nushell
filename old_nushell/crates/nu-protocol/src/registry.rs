use crate::{Signature, Value};
use nu_source::Spanned;
use std::fmt::Debug;

pub trait VariableRegistry {
    fn get_variable(&self, name: &Spanned<&str>) -> Option<Value>;
    fn variables(&self) -> Vec<String>;
}

pub trait SignatureRegistry: Debug {
    fn names(&self) -> Vec<String>;
    fn has(&self, name: &str) -> bool;
    fn get(&self, name: &str) -> Option<Signature>;
    fn clone_box(&self) -> Box<dyn SignatureRegistry>;
}

impl SignatureRegistry for Box<dyn SignatureRegistry> {
    fn names(&self) -> Vec<String> {
        (&**self).names()
    }

    fn has(&self, name: &str) -> bool {
        (&**self).has(name)
    }
    fn get(&self, name: &str) -> Option<Signature> {
        (&**self).get(name)
    }
    fn clone_box(&self) -> Box<dyn SignatureRegistry> {
        (&**self).clone_box()
    }
}
