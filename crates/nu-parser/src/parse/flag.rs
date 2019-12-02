use crate::hir::syntax_shape::flat_shape::FlatShape;
use derive_new::new;
use getset::Getters;
use nu_source::{b, DebugDocBuilder, PrettyDebugWithSource, Span, Spanned, SpannedItem};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Deserialize, Serialize)]
pub enum FlagKind {
    Shorthand,
    Longhand,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Getters, new)]
#[get = "pub(crate)"]
pub struct Flag {
    pub(crate) kind: FlagKind,
    pub(crate) name: Span,
    pub(crate) span: Span,
}

impl PrettyDebugWithSource for Flag {
    fn pretty_debug(&self, source: &str) -> DebugDocBuilder {
        let prefix = match self.kind {
            FlagKind::Longhand => b::description("--"),
            FlagKind::Shorthand => b::description("-"),
        };

        prefix + b::description(self.name.slice(source))
    }
}

impl Flag {
    pub fn color(&self) -> Spanned<FlatShape> {
        match self.kind {
            FlagKind::Longhand => FlatShape::Flag.spanned(self.span),
            FlagKind::Shorthand => FlatShape::ShorthandFlag.spanned(self.span),
        }
    }
}
