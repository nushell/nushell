use crate::prelude::*;
use derive_new::new;
use std::fmt::{self, Write};

pub struct Debuggable<'a, T: FormatDebug> {
    inner: &'a T,
    source: &'a str,
}

impl FormatDebug for str {
    fn fmt_debug(&self, f: &mut DebugFormatter, _source: &str) -> fmt::Result {
        write!(f, "{}", self)
    }
}

impl<T: ToDebug> fmt::Display for Debuggable<'_, T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.inner.fmt_debug(
            &mut DebugFormatter::new(
                f,
                ansi_term::Color::White.bold(),
                ansi_term::Color::Black.bold(),
            ),
            self.source,
        )
    }
}

pub trait HasTag {
    fn tag(&self) -> Tag;
}

#[derive(new)]
pub struct DebugFormatter<'me, 'args> {
    formatter: &'me mut std::fmt::Formatter<'args>,
    style: ansi_term::Style,
    default_style: ansi_term::Style,
}

impl<'me, 'args> DebugFormatter<'me, 'args> {
    pub fn say<'debuggable>(
        &mut self,
        kind: &str,
        debuggable: Debuggable<'debuggable, impl FormatDebug>,
    ) -> std::fmt::Result {
        write!(self, "{}", self.style.paint(kind))?;
        write!(self, "{}", self.default_style.paint(" "))?;
        write!(
            self,
            "{}",
            self.default_style.paint(format!("{}", debuggable))
        )
    }

    pub fn say_str<'debuggable>(
        &mut self,
        kind: &str,
        string: impl AsRef<str>,
    ) -> std::fmt::Result {
        write!(self, "{}", self.style.paint(kind))?;
        write!(self, "{}", self.default_style.paint(" "))?;
        write!(self, "{}", self.default_style.paint(string.as_ref()))
    }

    pub fn say_block(
        &mut self,
        kind: &str,
        block: impl FnOnce(&mut Self) -> std::fmt::Result,
    ) -> std::fmt::Result {
        write!(self, "{}", self.style.paint(kind))?;
        write!(self, "{}", self.default_style.paint(" "))?;
        block(self)
    }

    pub fn say_dict<'debuggable>(
        &mut self,
        kind: &str,
        dict: indexmap::IndexMap<&str, String>,
    ) -> std::fmt::Result {
        write!(self, "{}", self.style.paint(kind))?;
        write!(self, "{}", self.default_style.paint(" "))?;

        let last = dict.len() - 1;

        for (i, (key, value)) in dict.into_iter().enumerate() {
            write!(self, "{}", self.default_style.paint(key))?;
            write!(self, "{}", self.default_style.paint("=["))?;
            write!(self, "{}", self.style.paint(value))?;
            write!(self, "{}", self.default_style.paint("]"))?;

            if i != last {
                write!(self, "{}", self.default_style.paint(" "))?;
            }
        }

        Ok(())
    }
}

impl<'a, 'b> std::fmt::Write for DebugFormatter<'a, 'b> {
    fn write_str(&mut self, s: &str) -> std::fmt::Result {
        self.formatter.write_str(s)
    }

    fn write_char(&mut self, c: char) -> std::fmt::Result {
        self.formatter.write_char(c)
    }

    fn write_fmt(self: &mut Self, args: std::fmt::Arguments<'_>) -> std::fmt::Result {
        self.formatter.write_fmt(args)
    }
}

pub trait FormatDebug: std::fmt::Debug {
    fn fmt_debug(&self, f: &mut DebugFormatter, source: &str) -> fmt::Result;
}

pub trait ToDebug: Sized + FormatDebug {
    fn debug<'a>(&'a self, source: &'a str) -> Debuggable<'a, Self>;
}

impl FormatDebug for Box<dyn FormatDebug> {
    fn fmt_debug(&self, f: &mut DebugFormatter, source: &str) -> fmt::Result {
        (&**self).fmt_debug(f, source)
    }
}

impl<T> ToDebug for T
where
    T: FormatDebug + Sized,
{
    fn debug<'a>(&'a self, source: &'a str) -> Debuggable<'a, Self> {
        Debuggable {
            inner: self,
            source,
        }
    }
}
