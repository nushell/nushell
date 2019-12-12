use derive_new::new;
use getset::Getters;
use nu_source::{b, DebugDocBuilder, HasSpan, PrettyDebugWithSource, Span};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Deserialize, Serialize)]
pub enum CommentKind {
    Line,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Getters, new)]
pub struct Comment {
    pub(crate) kind: CommentKind,
    pub(crate) text: Span,
    pub(crate) span: Span,
}

impl Comment {
    pub fn line(text: impl Into<Span>, outer: impl Into<Span>) -> Comment {
        Comment {
            kind: CommentKind::Line,
            text: text.into(),
            span: outer.into(),
        }
    }
}

impl PrettyDebugWithSource for Comment {
    fn pretty_debug(&self, source: &str) -> DebugDocBuilder {
        let prefix = match self.kind {
            CommentKind::Line => b::description("#"),
        };

        prefix + b::description(self.text.slice(source))
    }
}

impl HasSpan for Comment {
    fn span(&self) -> Span {
        self.span
    }
}
