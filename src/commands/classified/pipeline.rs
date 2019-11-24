use crate::prelude::*;
use std::fmt;

use super::ClassifiedCommand;

#[derive(Debug, Clone)]
pub(crate) struct Pipeline {
    pub(crate) commands: Spanned<Vec<ClassifiedCommand>>,
}

impl FormatDebug for Pipeline {
    fn fmt_debug(&self, f: &mut DebugFormatter, source: &str) -> fmt::Result {
        f.say_str(
            "classified pipeline",
            self.commands.iter().map(|c| c.debug(source)).join(" | "),
        )
    }
}

impl HasSpan for Pipeline {
    fn span(&self) -> Span {
        self.commands.span
    }
}
