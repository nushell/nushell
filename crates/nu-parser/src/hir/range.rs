use crate::hir::SpannedExpression;

use derive_new::new;
use getset::Getters;
use nu_source::{b, DebugDocBuilder, PrettyDebugWithSource, Span};
use serde::{Deserialize, Serialize};

#[derive(
    Debug, Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Getters, Serialize, Deserialize, new,
)]
pub struct Range {
    #[get = "pub"]
    left: SpannedExpression,
    #[get = "pub"]
    dotdot: Span,
    #[get = "pub"]
    right: SpannedExpression,
}

impl PrettyDebugWithSource for Range {
    fn pretty_debug(&self, source: &str) -> DebugDocBuilder {
        b::delimit(
            "<",
            self.left.pretty_debug(source)
                + b::space()
                + b::keyword(self.dotdot.slice(source))
                + b::space()
                + self.right.pretty_debug(source),
            ">",
        )
        .group()
    }
}
