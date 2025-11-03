use crate::{RegId, Span};
use miette::Diagnostic;
use serde::{Deserialize, Serialize};
use thiserror::Error;

/// An internal compiler error, generally means a Nushell bug rather than an issue with user error
/// since parsing and typechecking has already passed.
#[derive(Debug, Clone, Error, Diagnostic, PartialEq, Serialize, Deserialize)]
pub enum CompileError {
    #[error("Register overflow.")]
    #[diagnostic(code(nu::compile::register_overflow))]
    RegisterOverflow {
        #[label("the code being compiled is probably too large")]
        block_span: Option<Span>,
    },

    #[error("Register {reg_id} was uninitialized when used, possibly reused.")]
    #[diagnostic(
        code(nu::compile::register_uninitialized),
        help(
            "this is a compiler bug. Please report it at https://github.com/nushell/nushell/issues/new\nfrom: {caller}"
        )
    )]
    RegisterUninitialized { reg_id: RegId, caller: String },

    #[error("Register {reg_id} was uninitialized when used, possibly reused.")]
    #[diagnostic(
        code(nu::compile::register_uninitialized),
        help(
            "this is a compiler bug. Please report it at https://github.com/nushell/nushell/issues/new\nfrom: {caller}"
        )
    )]
    RegisterUninitializedWhilePushingInstruction {
        reg_id: RegId,
        caller: String,
        instruction: String,
        #[label("while adding this instruction: {instruction}")]
        span: Span,
    },

    #[error("Block contains too much string data: maximum 4 GiB exceeded.")]
    #[diagnostic(
        code(nu::compile::data_overflow),
        help("try loading the string data from a file instead")
    )]
    DataOverflow {
        #[label("while compiling this block")]
        block_span: Option<Span>,
    },

    #[error("Block contains too many files.")]
    #[diagnostic(
        code(nu::compile::register_overflow),
        help("try using fewer file redirections")
    )]
    FileOverflow {
        #[label("while compiling this block")]
        block_span: Option<Span>,
    },

    #[error("Invalid redirect mode: File should not be specified by commands.")]
    #[diagnostic(
        code(nu::compile::invalid_redirect_mode),
        help(
            "this is a command bug. Please report it at https://github.com/nushell/nushell/issues/new"
        )
    )]
    InvalidRedirectMode {
        #[label("while compiling this expression")]
        span: Span,
    },

    #[error("Encountered garbage, likely due to parse error.")]
    #[diagnostic(code(nu::compile::garbage))]
    Garbage {
        #[label("garbage found here")]
        span: Span,
    },

    #[error("Unsupported operator expression.")]
    #[diagnostic(code(nu::compile::unsupported_operator_expression))]
    UnsupportedOperatorExpression {
        #[label("this expression is in operator position but is not an operator")]
        span: Span,
    },

    #[error("Attempted access of $env by integer path.")]
    #[diagnostic(code(nu::compile::access_env_by_int))]
    AccessEnvByInt {
        #[label("$env keys should be strings")]
        span: Span,
    },

    #[error("Encountered invalid `{keyword}` keyword call.")]
    #[diagnostic(code(nu::compile::invalid_keyword_call))]
    InvalidKeywordCall {
        keyword: String,
        #[label("this call is not properly formed")]
        span: Span,
    },

    #[error("Attempted to set branch target of non-branch instruction.")]
    #[diagnostic(
        code(nu::compile::set_branch_target_of_non_branch_instruction),
        help(
            "this is a compiler bug. Please report it at https://github.com/nushell/nushell/issues/new"
        )
    )]
    SetBranchTargetOfNonBranchInstruction {
        instruction: String,
        #[label("tried to modify: {instruction}")]
        span: Span,
    },

    /// You're trying to run an unsupported external command.
    ///
    /// ## Resolution
    ///
    /// Make sure there's an appropriate `run-external` declaration for this external command.
    #[error("External calls are not supported.")]
    #[diagnostic(
        code(nu::compile::run_external_not_found),
        help("`run-external` was not found in scope")
    )]
    RunExternalNotFound {
        #[label("can't be run in this context")]
        span: Span,
    },

    /// Invalid assignment left-hand side
    ///
    /// ## Resolution
    ///
    /// Assignment requires that you assign to a variable or variable cell path.
    #[error("Assignment operations require a variable.")]
    #[diagnostic(
        code(nu::compile::assignment_requires_variable),
        help("try assigning to a variable or a cell path of a variable")
    )]
    AssignmentRequiresVar {
        #[label("needs to be a variable")]
        span: Span,
    },

    /// Invalid assignment left-hand side
    ///
    /// ## Resolution
    ///
    /// Assignment requires that you assign to a mutable variable or cell path.
    #[error("Assignment to an immutable variable.")]
    #[diagnostic(
        code(nu::compile::assignment_requires_mutable_variable),
        help("declare the variable with `mut`, or shadow it again with `let`")
    )]
    AssignmentRequiresMutableVar {
        #[label("needs to be a mutable variable")]
        span: Span,
    },

    /// This environment variable cannot be set manually.
    ///
    /// ## Resolution
    ///
    /// This environment variable is set automatically by Nushell and cannot not be set manually.
    #[error("{envvar_name} cannot be set manually.")]
    #[diagnostic(
        code(nu::compile::automatic_env_var_set_manually),
        help(
            r#"The environment variable '{envvar_name}' is set automatically by Nushell and cannot be set manually."#
        )
    )]
    AutomaticEnvVarSetManually {
        envvar_name: String,
        #[label("cannot set '{envvar_name}' manually")]
        span: Span,
    },

    /// It is not possible to replace the entire environment at once
    ///
    /// ## Resolution
    ///
    /// Setting the entire environment is not allowed. Change environment variables individually
    /// instead.
    #[error("Cannot replace environment.")]
    #[diagnostic(
        code(nu::compile::cannot_replace_env),
        help("Assigning a value to '$env' is not allowed.")
    )]
    CannotReplaceEnv {
        #[label("setting '$env' not allowed")]
        span: Span,
    },

    #[error("Unexpected expression.")]
    #[diagnostic(code(nu::compile::unexpected_expression))]
    UnexpectedExpression {
        expr_name: String,
        #[label("{expr_name} is not allowed in this context")]
        span: Span,
    },

    #[error("Missing required declaration: `{decl_name}`")]
    #[diagnostic(code(nu::compile::missing_required_declaration))]
    MissingRequiredDeclaration {
        decl_name: String,
        #[label("`{decl_name}` must be in scope to compile this expression")]
        span: Span,
    },

    #[error("Invalid literal")]
    #[diagnostic(code(nu::compile::invalid_literal))]
    InvalidLiteral {
        msg: String,
        #[label("{msg}")]
        span: Span,
    },

    #[error("{msg}")]
    #[diagnostic(code(nu::compile::not_in_a_try))]
    NotInATry {
        msg: String,
        #[label("can't be used outside of a try block")]
        span: Option<Span>,
    },

    #[error("{msg}")]
    #[diagnostic(code(nu::compile::not_in_a_loop))]
    NotInALoop {
        msg: String,
        #[label("can't be used outside of a loop")]
        span: Option<Span>,
    },

    #[error("Incoherent loop state: the loop that ended was not the one we were expecting.")]
    #[diagnostic(
        code(nu::compile::incoherent_loop_state),
        help(
            "this is a compiler bug. Please report it at https://github.com/nushell/nushell/issues/new"
        )
    )]
    IncoherentLoopState {
        #[label("while compiling this block")]
        block_span: Option<Span>,
    },

    #[error("Undefined label `{label_id}`.")]
    #[diagnostic(
        code(nu::compile::undefined_label),
        help(
            "this is a compiler bug. Please report it at https://github.com/nushell/nushell/issues/new"
        )
    )]
    UndefinedLabel {
        label_id: usize,
        #[label("label was used while compiling this code")]
        span: Option<Span>,
    },
}
