use nu_protocol::ShellError;

use nu_protocol::Span;

use nu_protocol::RegId;

/// An internal compiler error, generally means a Nushell bug rather than an issue with user error
/// since parsing and typechecking has already passed.
#[derive(Debug)]
pub enum CompileError {
    RegisterOverflow,
    RegisterUninitialized(RegId),
    DataOverflow,
    InvalidRedirectMode,
    Garbage,
    UnsupportedOperatorExpression,
    AccessEnvByInt(Span),
    InvalidKeywordCall(&'static str, Span),
    SetBranchTargetOfNonBranchInstruction,
    InstructionIndexOutOfRange(usize),
    RunExternalNotFound,
    InvalidLhsForAssignment(Span),
    ModifyImmutableVariable(Span),
    Todo(&'static str),
}

impl CompileError {
    pub fn message(&self) -> String {
        match self {
            CompileError::RegisterOverflow => format!("register overflow"),
            CompileError::RegisterUninitialized(reg_id) => {
                format!("register {reg_id} is uninitialized when used, possibly reused")
            }
            CompileError::DataOverflow => {
                format!("block contains too much string data: maximum 4 GiB exceeded")
            }
            CompileError::InvalidRedirectMode => {
                "invalid redirect mode: File should not be specified by commands".into()
            }
            CompileError::Garbage => "encountered garbage, likely due to parse error".into(),
            CompileError::UnsupportedOperatorExpression => "unsupported operator expression".into(),
            CompileError::AccessEnvByInt(_) => "attempted access of $env by integer path".into(),
            CompileError::InvalidKeywordCall(kind, _) => format!("invalid `{kind}` keyword call"),
            CompileError::SetBranchTargetOfNonBranchInstruction => {
                "attempted to set branch target of non-branch instruction".into()
            }
            CompileError::InstructionIndexOutOfRange(index) => {
                format!("instruction index out of range: {index}")
            }
            CompileError::RunExternalNotFound => {
                "run-external is not supported here, so external calls can't be compiled".into()
            }
            CompileError::InvalidLhsForAssignment(_) => {
                "invalid left-hand side for assignment".into()
            }
            CompileError::ModifyImmutableVariable(_) => {
                "attempted to modify immutable variable".into()
            }
            CompileError::Todo(msg) => {
                format!("TODO: {msg}")
            }
        }
    }

    pub fn span(&self) -> Option<Span> {
        match self {
            CompileError::AccessEnvByInt(span)
            | CompileError::InvalidKeywordCall(_, span)
            | CompileError::InvalidLhsForAssignment(span)
            | CompileError::ModifyImmutableVariable(span) => Some(*span),
            _ => None,
        }
    }

    pub fn to_shell_error(self, span: Option<Span>) -> ShellError {
        ShellError::IrCompileError {
            msg: self.message(),
            span: self.span().or(span),
        }
    }
}
