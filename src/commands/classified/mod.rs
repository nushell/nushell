use crate::parser::{hir, TokenNode};
use crate::prelude::*;

mod dynamic;
mod external;
mod internal;
mod pipeline;

#[allow(unused_imports)]
pub(crate) use dynamic::Command as DynamicCommand;
#[allow(unused_imports)]
pub(crate) use external::{Command as ExternalCommand, ExternalArg, ExternalArgs, StreamNext};
pub(crate) use internal::Command as InternalCommand;
pub(crate) use pipeline::Pipeline as ClassifiedPipeline;

pub(crate) struct ClassifiedInputStream {
    pub(crate) objects: InputStream,
    pub(crate) stdin: Option<std::fs::File>,
}

impl ClassifiedInputStream {
    pub(crate) fn new() -> ClassifiedInputStream {
        ClassifiedInputStream {
            objects: vec![UntaggedValue::nothing().into_untagged_value()].into(),
            stdin: None,
        }
    }

    pub(crate) fn from_input_stream(stream: impl Into<InputStream>) -> ClassifiedInputStream {
        ClassifiedInputStream {
            objects: stream.into(),
            stdin: None,
        }
    }

    pub(crate) fn from_stdout(stdout: std::fs::File) -> ClassifiedInputStream {
        ClassifiedInputStream {
            objects: VecDeque::new().into(),
            stdin: Some(stdout),
        }
    }
}

#[derive(Debug, Clone, Eq, PartialEq)]
pub enum ClassifiedCommand {
    #[allow(unused)]
    Expr(TokenNode),
    #[allow(unused)]
    Dynamic(hir::Call),
    Internal(InternalCommand),
    External(ExternalCommand),
}

impl PrettyDebugWithSource for ClassifiedCommand {
    fn pretty_debug(&self, source: &str) -> DebugDocBuilder {
        match self {
            ClassifiedCommand::Expr(token) => b::typed("command", token.pretty_debug(source)),
            ClassifiedCommand::Dynamic(call) => b::typed("command", call.pretty_debug(source)),
            ClassifiedCommand::Internal(internal) => internal.pretty_debug(source),
            ClassifiedCommand::External(external) => external.pretty_debug(source),
        }
    }
}

impl HasSpan for ClassifiedCommand {
    fn span(&self) -> Span {
        match self {
            ClassifiedCommand::Expr(node) => node.span(),
            ClassifiedCommand::Internal(command) => command.span(),
            ClassifiedCommand::Dynamic(call) => call.span,
            ClassifiedCommand::External(command) => command.span(),
        }
    }
}
