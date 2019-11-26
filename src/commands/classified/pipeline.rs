use super::ClassifiedCommand;
use crate::prelude::*;

#[derive(Debug, Clone)]
pub(crate) struct Pipeline {
    pub(crate) commands: ClassifiedCommands,
}

impl Pipeline {
    pub fn commands(list: Vec<ClassifiedCommand>, span: impl Into<Span>) -> Pipeline {
        Pipeline {
            commands: ClassifiedCommands {
                list,
                span: span.into(),
            },
        }
    }
}

#[derive(Debug, Clone)]
pub struct ClassifiedCommands {
    pub list: Vec<ClassifiedCommand>,
    pub span: Span,
}

impl HasSpan for Pipeline {
    fn span(&self) -> Span {
        self.commands.span
    }
}

impl PrettyDebugWithSource for Pipeline {
    fn pretty_debug(&self, source: &str) -> DebugDocBuilder {
        b::intersperse(
            self.commands.list.iter().map(|c| c.pretty_debug(source)),
            b::operator(" | "),
        )
        .or(b::delimit("<", b::description("empty pipeline"), ">"))
    }
}
