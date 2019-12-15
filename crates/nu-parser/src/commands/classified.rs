pub mod external;
pub mod internal;

use crate::commands::classified::external::ExternalCommand;
use crate::commands::classified::internal::InternalCommand;
use crate::hir;
use crate::parse::token_tree::TokenNode;
use derive_new::new;
use nu_source::{b, DebugDocBuilder, HasSpan, PrettyDebugWithSource, Span};

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

#[derive(new, Debug, Eq, PartialEq)]
pub(crate) struct DynamicCommand {
    pub(crate) args: hir::Call,
}

#[derive(Debug, Clone)]
pub struct Commands {
    pub list: Vec<ClassifiedCommand>,
    pub span: Span,
}

impl std::ops::Deref for Commands {
    type Target = [ClassifiedCommand];

    fn deref(&self) -> &Self::Target {
        &self.list
    }
}

#[derive(Debug, Clone)]
pub struct ClassifiedPipeline {
    pub commands: Commands,
}

impl ClassifiedPipeline {
    pub fn commands(list: Vec<ClassifiedCommand>, span: impl Into<Span>) -> ClassifiedPipeline {
        ClassifiedPipeline {
            commands: Commands {
                list,
                span: span.into(),
            },
        }
    }
}

impl HasSpan for ClassifiedPipeline {
    fn span(&self) -> Span {
        self.commands.span
    }
}

impl PrettyDebugWithSource for ClassifiedPipeline {
    fn pretty_debug(&self, source: &str) -> DebugDocBuilder {
        b::intersperse(
            self.commands.iter().map(|c| c.pretty_debug(source)),
            b::operator(" | "),
        )
        .or(b::delimit("<", b::description("empty pipeline"), ">"))
    }
}
