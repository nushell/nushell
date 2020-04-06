use std::fmt::Debug;

use nu_source::{b, DebugDocBuilder, HasSpan, PrettyDebug, PrettyDebugWithSource, Span, Tag};

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

#[derive(Debug, Clone, Eq, PartialEq)]
pub struct ExternalArg {
    pub arg: String,
    pub tag: Tag,
}

impl ExternalArg {
    pub fn has(&self, name: &str) -> bool {
        self.arg == name
    }

    pub fn is_it(&self) -> bool {
        self.has("$it")
    }

    pub fn is_nu(&self) -> bool {
        self.has("$nu")
    }

    pub fn looks_like_it(&self) -> bool {
        self.arg.starts_with("$it") && (self.arg.starts_with("$it.") || self.is_it())
    }

    pub fn looks_like_nu(&self) -> bool {
        self.arg.starts_with("$nu") && (self.arg.starts_with("$nu.") || self.is_nu())
    }
}

impl std::ops::Deref for ExternalArg {
    type Target = str;

    fn deref(&self) -> &str {
        &self.arg
    }
}

#[derive(Debug, Clone, Eq, PartialEq)]
pub struct ExternalArgs {
    pub list: Vec<ExternalArg>,
    pub span: Span,
}

impl ExternalArgs {
    pub fn iter(&self) -> impl Iterator<Item = &ExternalArg> {
        self.list.iter()
    }
}

impl std::ops::Deref for ExternalArgs {
    type Target = [ExternalArg];

    fn deref(&self) -> &[ExternalArg] {
        &self.list
    }
}

#[derive(Debug, Clone, Eq, PartialEq)]
pub struct ExternalCommand {
    pub name: String,

    pub name_tag: Tag,
    pub args: ExternalArgs,
}

impl ExternalCommand {
    pub fn has_it_argument(&self) -> bool {
        self.args.iter().any(|arg| arg.looks_like_it())
    }

    pub fn has_nu_argument(&self) -> bool {
        self.args.iter().any(|arg| arg.looks_like_nu())
    }
}

impl PrettyDebug for ExternalCommand {
    fn pretty(&self) -> DebugDocBuilder {
        b::typed(
            "external command",
            b::description(&self.name)
                + b::preceded(
                    b::space(),
                    b::intersperse(
                        self.args.iter().map(|a| b::primitive(a.arg.to_string())),
                        b::space(),
                    ),
                ),
        )
    }
}

impl HasSpan for ExternalCommand {
    fn span(&self) -> Span {
        self.name_tag.span.until(self.args.span)
    }
}
