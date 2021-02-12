use crate::meta::Spanned;
use crate::term_colored::TermColored;
use crate::text::Text;
use derive_new::new;
use pretty::{BoxAllocator, DocAllocator};
use std::hash::Hash;
use termcolor::{Color, ColorSpec};

#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd, Hash)]
pub enum ShellStyle {
    Delimiter,
    Key,
    Value,
    Equals,
    Kind,
    Keyword,
    Operator,
    Variable,
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
                .set_fg(Some(Color::Green))
                .set_intense(true)
                .clone(),
            ShellStyle::Value => ColorSpec::new()
                .set_fg(Some(Color::White))
                .set_intense(true)
                .clone(),
            ShellStyle::Equals => ColorSpec::new()
                .set_fg(Some(Color::Green))
                .set_intense(true)
                .clone(),
            ShellStyle::Kind => ColorSpec::new().set_fg(Some(Color::Cyan)).clone(),
            ShellStyle::Variable => ColorSpec::new()
                .set_fg(Some(Color::Green))
                .set_intense(true)
                .clone(),
            ShellStyle::Keyword => ColorSpec::new().set_fg(Some(Color::Magenta)).clone(),
            ShellStyle::Operator => ColorSpec::new().set_fg(Some(Color::Yellow)).clone(),
            ShellStyle::Primitive => ColorSpec::new()
                .set_fg(Some(Color::Green))
                .set_intense(true)
                .clone(),
            ShellStyle::Opaque => ColorSpec::new()
                .set_fg(Some(Color::Yellow))
                .set_intense(true)
                .clone(),
            ShellStyle::Description => ColorSpec::new()
                .set_fg(Some(Color::Green))
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

pub use self::DebugDocBuilder as DbgDocBldr;

#[derive(Clone, new)]
pub struct DebugDocBuilder {
    pub inner: PrettyDebugDocBuilder,
}

impl PrettyDebug for bool {
    fn pretty(&self) -> DebugDocBuilder {
        match self {
            true => DbgDocBldr::primitive("true"),
            false => DbgDocBldr::primitive("false"),
        }
    }
}

impl PrettyDebug for () {
    fn pretty(&self) -> DebugDocBuilder {
        DbgDocBldr::primitive("nothing")
    }
}

impl PrettyDebug for DebugDocBuilder {
    fn pretty(&self) -> DebugDocBuilder {
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

    pub fn into_value(self) -> DebugDocBuilder {
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

    pub fn into_kind(self) -> DebugDocBuilder {
        self.inner
            .annotate(ShellAnnotation::style(ShellStyle::Kind))
            .into()
    }

    pub fn typed(kind: &str, value: DebugDocBuilder) -> DebugDocBuilder {
        DbgDocBldr::kind(kind) + DbgDocBldr::delimit("[", value.group(), "]")
    }

    pub fn subtyped(
        kind: &str,
        subkind: impl std::fmt::Display,
        value: DebugDocBuilder,
    ) -> DebugDocBuilder {
        DbgDocBldr::delimit(
            "(",
            (DbgDocBldr::kind(kind)
                + DbgDocBldr::delimit("[", DbgDocBldr::kind(format!("{}", subkind)), "]"))
            .group()
                + DbgDocBldr::space()
                + value.group(),
            ")",
        )
        .group()
    }

    pub fn keyword(string: impl std::fmt::Display) -> DebugDocBuilder {
        DebugDocBuilder::styled(string, ShellStyle::Keyword)
    }

    pub fn var(string: impl std::fmt::Display) -> DebugDocBuilder {
        DebugDocBuilder::styled(string, ShellStyle::Variable)
    }

    pub fn operator(string: impl std::fmt::Display) -> DebugDocBuilder {
        DebugDocBuilder::styled(string, ShellStyle::Operator)
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

    pub fn preceded(before: DebugDocBuilder, body: DebugDocBuilder) -> DebugDocBuilder {
        if body.is_empty() {
            body
        } else {
            before + body
        }
    }

    pub fn surrounded_option(
        before: Option<DebugDocBuilder>,
        builder: Option<DebugDocBuilder>,
        after: Option<DebugDocBuilder>,
    ) -> DebugDocBuilder {
        match builder {
            None => DebugDocBuilder::blank(),
            Some(b) => DbgDocBldr::option(before) + b + DbgDocBldr::option(after),
        }
    }

    pub fn preceded_option(
        before: Option<DebugDocBuilder>,
        builder: Option<DebugDocBuilder>,
    ) -> DebugDocBuilder {
        DebugDocBuilder::surrounded_option(before, builder, None)
    }

    pub fn option(builder: Option<DebugDocBuilder>) -> DebugDocBuilder {
        builder.unwrap_or_else(DebugDocBuilder::blank)
    }

    pub fn space() -> DebugDocBuilder {
        BoxAllocator.space().into()
    }

    pub fn newline() -> DebugDocBuilder {
        BoxAllocator.newline().into()
    }

    pub fn is_empty(&self) -> bool {
        matches!(&self.inner.1, pretty::Doc::Nil)
    }

    pub fn or(self, doc: DebugDocBuilder) -> DebugDocBuilder {
        if self.is_empty() {
            doc
        } else {
            self
        }
    }

    pub fn group(self) -> DebugDocBuilder {
        self.inner.group().into()
    }

    pub fn nest(self) -> DebugDocBuilder {
        self.inner.nest(1).group().into()
    }

    pub fn intersperse_with_source<'a, T: PrettyDebugWithSource + 'a>(
        list: impl IntoIterator<Item = &'a T>,
        separator: DebugDocBuilder,
        source: &str,
    ) -> DebugDocBuilder {
        BoxAllocator
            .intersperse(
                list.into_iter().filter_map(|item| {
                    let item = item.pretty_debug(source);
                    if item.is_empty() {
                        None
                    } else {
                        Some(item)
                    }
                }),
                separator,
            )
            .into()
    }

    pub fn intersperse<T: PrettyDebug>(
        list: impl IntoIterator<Item = T>,
        separator: DebugDocBuilder,
    ) -> DebugDocBuilder {
        BoxAllocator
            .intersperse(
                list.into_iter().filter_map(|item| {
                    let item = item.pretty();
                    if item.is_empty() {
                        None
                    } else {
                        Some(item)
                    }
                }),
                separator,
            )
            .into()
    }

    pub fn list(list: impl IntoIterator<Item = DebugDocBuilder>) -> DebugDocBuilder {
        let mut result: DebugDocBuilder = BoxAllocator.nil().into();

        for item in list {
            result = result + item;
        }

        result
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

#[derive(Debug, Copy, Clone)]
pub enum PrettyDebugRefineKind {
    ContextFree,
    WithContext,
}

pub trait PrettyDebugWithSource: Sized {
    fn pretty_debug(&self, source: &str) -> DebugDocBuilder;

    fn refined_pretty_debug(
        &self,
        _refine: PrettyDebugRefineKind,
        source: &str,
    ) -> DebugDocBuilder {
        self.pretty_debug(source)
    }

    // This is a transitional convenience method
    fn debug(&self, source: impl Into<Text>) -> String
    where
        Self: Clone,
    {
        self.clone().debuggable(source).display()
    }

    fn debuggable(self, source: impl Into<Text>) -> DebuggableWithSource<Self> {
        DebuggableWithSource {
            inner: self,
            source: source.into(),
        }
    }
}

impl<T: PrettyDebug> PrettyDebug for Spanned<T> {
    fn pretty(&self) -> DebugDocBuilder {
        self.item.pretty()
    }
}

impl<T: PrettyDebug> PrettyDebugWithSource for T {
    fn pretty_debug(&self, _source: &str) -> DebugDocBuilder {
        self.pretty()
    }
}

impl<T: PrettyDebugWithSource, E> PrettyDebugWithSource for Result<T, E> {
    fn pretty_debug(&self, source: &str) -> DebugDocBuilder {
        match self {
            Err(_) => DbgDocBldr::error("error"),
            Ok(val) => val.pretty_debug(source),
        }
    }
}

pub struct DebuggableWithSource<T: PrettyDebugWithSource> {
    inner: T,
    source: Text,
}

impl<T> PrettyDebug for DebuggableWithSource<T>
where
    T: PrettyDebugWithSource,
{
    fn pretty(&self) -> DebugDocBuilder {
        self.inner.pretty_debug(&self.source)
    }
}

impl PrettyDebug for DebugDoc {
    fn pretty(&self) -> DebugDocBuilder {
        DebugDocBuilder::new(BoxAllocator.nil().append(self.inner.clone()))
    }
}

pub trait PrettyDebug {
    fn pretty(&self) -> DebugDocBuilder;

    fn to_doc(&self) -> DebugDoc {
        DebugDoc::new(self.pretty().into())
    }

    fn pretty_doc(&self) -> PrettyDebugDoc {
        let builder = self.pretty();
        builder.inner.into()
    }

    fn pretty_builder(&self) -> PrettyDebugDocBuilder {
        let doc = self.pretty();
        doc.inner
    }

    /// A convenience method that prints out the document without colors in
    /// 70 columns. Generally, you should use plain_string or colored_string
    /// if possible, but display() can be useful for trace lines and things
    /// like that, where you don't have control over the terminal.
    fn display(&self) -> String {
        self.plain_string(70)
    }

    fn plain_string(&self, width: usize) -> String {
        let doc = self.pretty_doc();
        let mut buffer = termcolor::Buffer::no_color();

        let _ = doc.render_raw(width, &mut TermColored::new(&mut buffer));

        String::from_utf8_lossy(buffer.as_slice()).to_string()
    }

    fn colored_string(&self, width: usize) -> String {
        let doc = self.pretty_doc();
        let mut buffer = termcolor::Buffer::ansi();

        let _ = doc.render_raw(width, &mut TermColored::new(&mut buffer));

        String::from_utf8_lossy(buffer.as_slice()).to_string()
    }
}

impl From<PrettyDebugDocBuilder> for DebugDocBuilder {
    fn from(x: PrettyDebugDocBuilder) -> Self {
        DebugDocBuilder { inner: x }
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

impl From<DebugDocBuilder> for PrettyDebugDoc {
    fn from(x: DebugDocBuilder) -> Self {
        x.inner.into()
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

#[allow(clippy::derive_hash_xor_eq)]
impl std::hash::Hash for DebugDoc {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        hash_doc(&self.inner, state);
    }
}
