use crate::prelude::*;
use derive_new::new;
use getset::Getters;
use pretty::{BoxAllocator, DocAllocator};
use std::fmt::{self, Write};
use std::hash::Hash;
use termcolor::{Color, ColorSpec};

pub trait ShellTypeName {
    fn type_name(&self) -> &'static str;
}

impl<T: ShellTypeName> ShellTypeName for &T {
    fn type_name(&self) -> &'static str {
        (*self).type_name()
    }
}

pub trait SpannedTypeName {
    fn spanned_type_name(&self) -> Spanned<&'static str>;
}

impl<T: ShellTypeName> SpannedTypeName for Spanned<T> {
    fn spanned_type_name(&self) -> Spanned<&'static str> {
        self.item.type_name().spanned(self.span)
    }
}

impl<T: ShellTypeName> SpannedTypeName for Tagged<T> {
    fn spanned_type_name(&self) -> Spanned<&'static str> {
        self.item.type_name().spanned(self.tag.span)
    }
}

pub struct Debuggable<'a, T: FormatDebug> {
    inner: &'a T,
    source: &'a str,
}

impl FormatDebug for str {
    fn fmt_debug(&self, f: &mut DebugFormatter, _source: &str) -> fmt::Result {
        write!(f, "{}", self)
    }
}

impl<T: ToDebug> fmt::Debug for Debuggable<'_, T> {
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

impl<T: ToDebug> fmt::Display for Debuggable<'_, T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.inner.fmt_display(
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

#[derive(Getters, new)]
pub struct DebugFormatter<'me, 'args> {
    formatter: &'me mut std::fmt::Formatter<'args>,
    style: ansi_term::Style,
    default_style: ansi_term::Style,
}

impl<'me, 'args> DebugFormatter<'me, 'args> {
    pub fn say_simple(&mut self, kind: &str) -> std::fmt::Result {
        write!(self, "{}", self.style.paint(kind))
    }

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

    pub fn say_list<T, U: IntoIterator<Item = T>>(
        &mut self,
        kind: &str,
        list: U,
        open: impl Fn(&mut Self) -> std::fmt::Result,
        mut block: impl FnMut(&mut Self, &T) -> std::fmt::Result,
        interleave: impl Fn(&mut Self) -> std::fmt::Result,
        close: impl Fn(&mut Self) -> std::fmt::Result,
    ) -> std::fmt::Result {
        write!(self, "{}", self.style.paint(kind))?;
        write!(self, "{}", self.default_style.paint(" "))?;
        open(self)?;
        write!(self, " ")?;

        let mut list = list.into_iter();

        let first = match list.next() {
            None => return Ok(()),
            Some(first) => first,
        };

        block(self, &first)?;

        for item in list {
            interleave(self)?;
            block(self, &item)?;
        }

        write!(self, " ")?;
        close(self)?;

        Ok(())
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

#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd, Hash)]
pub enum ShellStyle {
    Delimiter,
    Key,
    Value,
    Equals,
    Kind,
    Keyword,
    Primitive,
    Opaque,
    Description,
    Error,
}

impl From<ShellAnnotation> for ColorSpec {
    fn from(ann: ShellAnnotation) -> ColorSpec {
        match ann.style {
            ShellStyle::Delimiter => ColorSpec::new()
                .set_fg(Some(Color::White))
                .set_intense(false)
                .clone(),
            ShellStyle::Key => ColorSpec::new()
                .set_fg(Some(Color::Black))
                .set_intense(true)
                .clone(),
            ShellStyle::Value => ColorSpec::new()
                .set_fg(Some(Color::White))
                .set_intense(true)
                .clone(),
            ShellStyle::Equals => ColorSpec::new()
                .set_fg(Some(Color::Black))
                .set_intense(true)
                .clone(),
            ShellStyle::Kind => ColorSpec::new().set_fg(Some(Color::Cyan)).clone(),
            ShellStyle::Keyword => ColorSpec::new().set_fg(Some(Color::Magenta)).clone(),
            ShellStyle::Primitive => ColorSpec::new()
                .set_fg(Some(Color::Green))
                .set_intense(true)
                .clone(),
            ShellStyle::Opaque => ColorSpec::new()
                .set_fg(Some(Color::Yellow))
                .set_intense(true)
                .clone(),
            ShellStyle::Description => ColorSpec::new()
                .set_fg(Some(Color::Black))
                .set_intense(true)
                .clone(),
            ShellStyle::Error => ColorSpec::new()
                .set_fg(Some(Color::Red))
                .set_intense(true)
                .clone(),
        }
    }
}

#[derive(Clone, Copy, Eq, Ord, PartialEq, PartialOrd, Hash, new)]
pub struct ShellAnnotation {
    style: ShellStyle,
}

impl std::fmt::Debug for ShellAnnotation {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:?}", self.style)
    }
}

impl ShellAnnotation {
    pub fn style(style: impl Into<ShellStyle>) -> ShellAnnotation {
        ShellAnnotation {
            style: style.into(),
        }
    }
}

pub type PrettyDebugDoc =
    pretty::Doc<'static, pretty::BoxDoc<'static, ShellAnnotation>, ShellAnnotation>;

pub type PrettyDebugDocBuilder = pretty::DocBuilder<'static, pretty::BoxAllocator, ShellAnnotation>;

#[derive(Clone, new)]
pub struct DebugDocBuilder {
    pub inner: PrettyDebugDocBuilder,
}

impl PrettyDebug for DebugDocBuilder {
    fn pretty_debug(&self) -> DebugDocBuilder {
        self.clone()
    }
}

impl std::ops::Add for DebugDocBuilder {
    type Output = DebugDocBuilder;

    fn add(self, rhs: DebugDocBuilder) -> DebugDocBuilder {
        DebugDocBuilder::new(self.inner.append(rhs.inner))
    }
}

impl DebugDocBuilder {
    pub fn from_doc(doc: DebugDoc) -> DebugDocBuilder {
        DebugDocBuilder {
            inner: BoxAllocator.nil().append(doc),
        }
    }

    pub fn blank() -> DebugDocBuilder {
        BoxAllocator.nil().into()
    }

    pub fn delimiter(string: impl std::fmt::Display) -> DebugDocBuilder {
        DebugDocBuilder::styled(string, ShellStyle::Delimiter)
    }

    pub fn key(string: impl std::fmt::Display) -> DebugDocBuilder {
        DebugDocBuilder::styled(string, ShellStyle::Key)
    }

    pub fn value(string: impl std::fmt::Display) -> DebugDocBuilder {
        DebugDocBuilder::styled(string, ShellStyle::Value)
    }

    pub fn as_value(self) -> DebugDocBuilder {
        self.inner
            .annotate(ShellAnnotation::style(ShellStyle::Value))
            .into()
    }

    pub fn equals() -> DebugDocBuilder {
        DebugDocBuilder::styled("=", ShellStyle::Equals)
    }

    pub fn kind(string: impl std::fmt::Display) -> DebugDocBuilder {
        DebugDocBuilder::styled(string, ShellStyle::Kind)
    }

    pub fn as_kind(self) -> DebugDocBuilder {
        self.inner
            .annotate(ShellAnnotation::style(ShellStyle::Kind))
            .into()
    }

    pub fn keyword(string: impl std::fmt::Display) -> DebugDocBuilder {
        DebugDocBuilder::styled(string, ShellStyle::Keyword)
    }

    pub fn primitive(string: impl std::fmt::Display) -> DebugDocBuilder {
        DebugDocBuilder::styled(format!("{}", string), ShellStyle::Primitive)
    }

    pub fn opaque(string: impl std::fmt::Display) -> DebugDocBuilder {
        DebugDocBuilder::styled(string, ShellStyle::Opaque)
    }

    pub fn description(string: impl std::fmt::Display) -> DebugDocBuilder {
        DebugDocBuilder::styled(string, ShellStyle::Description)
    }

    pub fn error(string: impl std::fmt::Display) -> DebugDocBuilder {
        DebugDocBuilder::styled(string, ShellStyle::Error)
    }

    pub fn delimit(start: &str, doc: DebugDocBuilder, end: &str) -> DebugDocBuilder {
        DebugDocBuilder::delimiter(start) + doc + DebugDocBuilder::delimiter(end)
    }

    pub fn space() -> DebugDocBuilder {
        BoxAllocator.space().into()
    }

    pub fn newline() -> DebugDocBuilder {
        BoxAllocator.newline().into()
    }

    pub fn group(self) -> DebugDocBuilder {
        self.inner.group().into()
    }

    pub fn nest(self) -> DebugDocBuilder {
        self.inner.nest(1).group().into()
    }

    pub fn intersperse(
        list: impl IntoIterator<Item = DebugDocBuilder>,
        separator: DebugDocBuilder,
    ) -> DebugDocBuilder {
        BoxAllocator.intersperse(list, separator).into()
    }

    pub fn list(list: impl IntoIterator<Item = DebugDocBuilder>) -> DebugDocBuilder {
        let mut result: DebugDocBuilder = BoxAllocator.nil().into();

        for item in list {
            result = result + item;
        }

        result.into()
    }

    fn styled(string: impl std::fmt::Display, style: ShellStyle) -> DebugDocBuilder {
        BoxAllocator
            .text(string.to_string())
            .annotate(ShellAnnotation::style(style))
            .into()
    }
}

impl std::ops::Deref for DebugDocBuilder {
    type Target = PrettyDebugDocBuilder;

    fn deref(&self) -> &Self::Target {
        &self.inner
    }
}

#[derive(Clone, Debug, Eq, Ord, PartialEq, PartialOrd, new)]
pub struct DebugDoc {
    pub inner: PrettyDebugDoc,
}

pub trait PrettyDebug {
    fn pretty_debug(&self) -> DebugDocBuilder;

    fn to_doc(&self) -> DebugDoc {
        DebugDoc::new(self.pretty_doc())
    }

    fn pretty_doc(&self) -> PrettyDebugDoc {
        let builder = self.pretty_debug();
        builder.inner.into()
    }

    fn pretty_builder(&self) -> PrettyDebugDocBuilder {
        let doc = self.pretty_debug();
        doc.inner
    }

    fn plain_string(&self, width: usize) -> String {
        let doc = self.pretty_doc();
        let mut buffer = termcolor::Buffer::no_color();

        doc.render_raw(
            width,
            &mut crate::parser::debug::TermColored::new(&mut buffer),
        )
        .unwrap();

        String::from_utf8_lossy(buffer.as_slice()).to_string()
    }

    fn colored_string(&self, width: usize) -> String {
        let doc = self.pretty_doc();
        let mut buffer = termcolor::Buffer::ansi();

        doc.render_raw(
            width,
            &mut crate::parser::debug::TermColored::new(&mut buffer),
        )
        .unwrap();

        String::from_utf8_lossy(buffer.as_slice()).to_string()
    }
}

impl Into<DebugDocBuilder> for PrettyDebugDocBuilder {
    fn into(self) -> DebugDocBuilder {
        DebugDocBuilder { inner: self }
    }
}

impl std::ops::Deref for DebugDoc {
    type Target = PrettyDebugDoc;

    fn deref(&self) -> &Self::Target {
        &self.inner
    }
}

impl From<DebugDoc> for PrettyDebugDoc {
    fn from(input: DebugDoc) -> PrettyDebugDoc {
        input.inner
    }
}

impl Into<PrettyDebugDoc> for DebugDocBuilder {
    fn into(self) -> PrettyDebugDoc {
        self.inner.into()
    }
}

fn hash_doc<H: std::hash::Hasher>(doc: &PrettyDebugDoc, state: &mut H) {
    match doc {
        pretty::Doc::Nil => 0u8.hash(state),
        pretty::Doc::Append(a, b) => {
            1u8.hash(state);
            hash_doc(&*a, state);
            hash_doc(&*b, state);
        }
        pretty::Doc::Group(a) => {
            2u8.hash(state);
            hash_doc(&*a, state);
        }
        pretty::Doc::Nest(a, b) => {
            3u8.hash(state);
            a.hash(state);
            hash_doc(&*b, state);
        }
        pretty::Doc::Space => 4u8.hash(state),
        pretty::Doc::Newline => 5u8.hash(state),
        pretty::Doc::Text(t) => {
            6u8.hash(state);
            t.hash(state);
        }
        pretty::Doc::Annotated(a, b) => {
            7u8.hash(state);
            a.hash(state);
            hash_doc(&*b, state);
        }
    }
}

impl std::hash::Hash for DebugDoc {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        hash_doc(&self.inner, state);
    }
}

pub trait PrettyType {
    fn pretty_type(&self) -> DebugDocBuilder;
}

pub trait FormatDebug: std::fmt::Debug {
    fn fmt_debug(&self, f: &mut DebugFormatter, source: &str) -> fmt::Result;

    fn fmt_display(&self, f: &mut DebugFormatter, source: &str) -> fmt::Result {
        self.fmt_debug(f, source)
    }
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
