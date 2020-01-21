use derive_new::new;
use getset::Getters;
use nu_source::{b, DebugDocBuilder, PrettyDebugWithSource, Span};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Deserialize, Serialize)]
pub enum CommentKind {
    Line,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Getters, new)]
pub struct Comment {
    pub(crate) kind: CommentKind,
    pub(crate) text: Span,
}

impl Comment {
    pub fn line(text: impl Into<Span>) -> Comment {
        Comment {
            kind: CommentKind::Line,
            text: text.into(),
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
