use crate::prelude::*;
use std::fmt;

pub struct Debuggable<'a, T: ToDebug> {
    inner: &'a T,
    source: &'a str,
}

impl<T: ToDebug> fmt::Display for Debuggable<'a, T> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        self.inner.fmt_debug(f, self.source)
    }
}

pub trait HasSpan {
    fn span(&self) -> Span;
}

pub trait ToDebug: Sized {
    fn debug(&'a self, source: &'a str) -> Debuggable<'a, Self> {
        Debuggable {
            inner: self,
            source,
        }
    }

    fn fmt_debug(&self, f: &mut fmt::Formatter, source: &str) -> fmt::Result;
}
