use nu_engine::CallExt;
use nu_protocol::{
    DeclId, FromValue, ShellError, Span,
    engine::{Call, EngineState, Stack, StateWorkingSet},
};

/// A helper utility to aid in implementing commands which have the same behavior for `run` and `run_const`.
///
/// Only supports functions in [`Call`] and [`CallExt`] which have a `const` suffix.
///
/// To use, the actual command logic should be moved to a function. Then, `eval` and `eval_const` can be implemented like this:
/// ```rust
/// # use nu_engine::command_prelude::*;
/// # use nu_cmd_base::WrapCall;
/// # fn do_command_logic(call: WrapCall) -> Result<PipelineData, ShellError> { Ok(PipelineData::empty()) }
///
/// # struct Command {}
/// # impl Command {
/// fn run(&self, engine_state: &EngineState, stack: &mut Stack, call: &Call) -> Result<PipelineData, ShellError> {
///     let call = WrapCall::Eval(engine_state, stack, call);
///     do_command_logic(call)
/// }
///
/// fn run_const(&self, working_set: &StateWorkingSet, call: &Call) -> Result<PipelineData, ShellError> {
///     let call = WrapCall::ConstEval(working_set, call);
///     do_command_logic(call)
/// }
/// # }
/// ```
///
/// Then, the typical [`Call`] and [`CallExt`] operations can be called using destructuring:
///
/// ```rust
/// # use nu_engine::command_prelude::*;
/// # use nu_cmd_base::WrapCall;
/// # let call = WrapCall::Eval(&EngineState::new(), &mut Stack::new(), &Call::new(Span::unknown()));
/// # fn do_command_logic(call: WrapCall) -> Result<(), ShellError> {
/// let (call, required): (_, String) = call.req(0)?;
/// let (call, flag): (_, Option<i64>) = call.get_flag("number")?;
/// # Ok(())
/// # }
/// ```
///
/// A new `WrapCall` instance has to be returned after each function to ensure
/// that there is only ever one copy of mutable [`Stack`] reference.
pub enum WrapCall<'a> {
    Eval(&'a EngineState, &'a mut Stack, &'a Call<'a>),
    ConstEval(&'a StateWorkingSet<'a>, &'a Call<'a>),
}

/// Macro to choose between the non-const and const versions of each [`Call`]/[`CallExt`] function
macro_rules! proxy {
    ($self:ident , $eval:ident , $const:ident , $( $args:expr ),*) => {
        match $self {
            WrapCall::Eval(engine_state, stack, call) => {
                Call::$eval(call, engine_state, stack, $( $args ),*)
                .map(|val| (WrapCall::Eval(engine_state, stack, call), val))
            },
            WrapCall::ConstEval(working_set, call) => {
                Call::$const(call, working_set, $( $args ),*)
                .map(|val| (WrapCall::ConstEval(working_set, call), val))
            },
        }
    };
}

impl WrapCall<'_> {
    pub fn head(&self) -> Span {
        match self {
            WrapCall::Eval(_, _, call) => call.head,
            WrapCall::ConstEval(_, call) => call.head,
        }
    }

    pub fn decl_id(&self) -> DeclId {
        match self {
            WrapCall::Eval(_, _, call) => call.decl_id,
            WrapCall::ConstEval(_, call) => call.decl_id,
        }
    }

    pub fn has_flag<T: FromValue>(self, flag_name: &str) -> Result<(Self, bool), ShellError> {
        proxy!(self, has_flag, has_flag_const, flag_name)
    }

    pub fn get_flag<T: FromValue>(self, name: &str) -> Result<(Self, Option<T>), ShellError> {
        proxy!(self, get_flag, get_flag_const, name)
    }

    pub fn req<T: FromValue>(self, pos: usize) -> Result<(Self, T), ShellError> {
        proxy!(self, req, req_const, pos)
    }

    pub fn rest<T: FromValue>(self, pos: usize) -> Result<(Self, Vec<T>), ShellError> {
        proxy!(self, rest, rest_const, pos)
    }

    pub fn opt<T: FromValue>(self, pos: usize) -> Result<(Self, Option<T>), ShellError> {
        proxy!(self, opt, opt_const, pos)
    }
}
