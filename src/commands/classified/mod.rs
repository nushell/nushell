use crate::parser::{hir, TokenNode};
use crate::prelude::*;
use std::fmt;

mod dynamic;
mod external;
mod internal;
mod pipeline;

#[allow(unused_imports)]
pub(crate) use dynamic::Command as DynamicCommand;
#[allow(unused_imports)]
pub(crate) use external::{Command as ExternalCommand, StreamNext};
pub(crate) use internal::Command as InternalCommand;
pub(crate) use pipeline::Pipeline as ClassifiedPipeline;

pub(crate) struct ClassifiedInputStream {
    pub(crate) objects: InputStream,
    pub(crate) stdin: Option<std::fs::File>,
}

impl ClassifiedInputStream {
    pub(crate) fn new() -> ClassifiedInputStream {
        ClassifiedInputStream {
            objects: vec![Value::nothing().tagged(Tag::unknown())].into(),
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
pub(crate) enum ClassifiedCommand {
    #[allow(unused)]
    Expr(TokenNode),
    Internal(InternalCommand),
    #[allow(unused)]
    Dynamic(Spanned<hir::Call>),
    External(ExternalCommand),
}

impl FormatDebug for ClassifiedCommand {
    fn fmt_debug(&self, f: &mut DebugFormatter, source: &str) -> fmt::Result {
        match self {
            ClassifiedCommand::Expr(expr) => expr.fmt_debug(f, source),
            ClassifiedCommand::Internal(internal) => internal.fmt_debug(f, source),
            ClassifiedCommand::Dynamic(dynamic) => dynamic.fmt_debug(f, source),
            ClassifiedCommand::External(external) => external.fmt_debug(f, source),
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
