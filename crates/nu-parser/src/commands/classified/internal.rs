use crate::hir;

use derive_new::new;
use nu_source::{b, DebugDocBuilder, HasSpan, PrettyDebugWithSource, Span, Tag};

#[derive(new, Debug, Clone, Eq, PartialEq)]
pub struct InternalCommand {
    pub name: String,
    pub name_tag: Tag,
    pub args: hir::Call,
}

impl PrettyDebugWithSource for InternalCommand {
    fn pretty_debug(&self, source: &str) -> DebugDocBuilder {
        b::typed(
            "internal command",
            b::description(&self.name) + b::space() + self.args.pretty_debug(source),
        )
    }
}

impl HasSpan for InternalCommand {
    fn span(&self) -> Span {
        let start = self.name_tag.span;

        start.until(self.args.span)
    }
}
