use std::fmt::Debug;

use nu_source::{DebugDocBuilder, HasSpan, PrettyDebugWithSource, Span};

pub trait SignatureRegistry: Debug {
    fn has(&self, name: &str) -> bool;
    fn get(&self, name: &str) -> Option<nu_protocol::Signature>;
    fn clone_box(&self) -> Box<dyn SignatureRegistry>;
}

impl SignatureRegistry for Box<dyn SignatureRegistry> {
    fn has(&self, name: &str) -> bool {
        (&**self).has(name)
    }
    fn get(&self, name: &str) -> Option<nu_protocol::Signature> {
        (&**self).get(name)
    }
    fn clone_box(&self) -> Box<dyn SignatureRegistry> {
        (&**self).clone_box()
    }
}

#[derive(Debug, Clone)]
pub struct Signature {
    pub(crate) unspanned: nu_protocol::Signature,
    span: Span,
}

impl Signature {
    pub fn new(unspanned: nu_protocol::Signature, span: impl Into<Span>) -> Signature {
        Signature {
            unspanned,
            span: span.into(),
        }
    }
}

impl HasSpan for Signature {
    fn span(&self) -> Span {
        self.span
    }
}

impl PrettyDebugWithSource for Signature {
    fn pretty_debug(&self, source: &str) -> DebugDocBuilder {
        self.unspanned.pretty_debug(source)
    }
}
