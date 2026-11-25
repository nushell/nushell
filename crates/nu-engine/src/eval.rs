use crate::named_flags::{
    can_rest_spread, expand_flag_record, find_signature_flag, list_spread_before_required_error,
    list_spread_without_rest_error,
};
use crate::{
    EvalBlockWithEarlyReturnFn,
    eval_ir::{eval_ir_block, expect_named_var_id},
};
use nu_protocol::{
    CompareTypes, ENV_VARIABLE_ID, PipelineData, PipelineExecutionData, ShellError, Signature,
    Span, Value, VarId,
    ast::Block,
    debugger::{DebugContext, WithDebug, WithoutDebug},
    engine::{Argument as EngineArgument, EngineState, EnvName, EnvVars, Stack},
};
use std::borrow::Cow;
use std::collections::{HashMap, HashSet};
use std::sync::Arc;

/// [`CallEval`] is used to evaluate a command or closure call.
///
/// It is intended as an internal interface in the engine, to make sure
/// command and closure calls behave the same.
/// If you want to evaluate a closure in a command, use [`ClosureEval`] or
/// [`ClosureEvalOnce`]. If you want to call a command, use [`eval_ir_call`](crate::eval_ir_call).
///
/// [`CallEval`] has a builder API.
/// It is first created vial [`CallEval::new`],
/// then has arguments added via [`CallEval::add_positional`] and [`CallEval::add_named`],
/// and then can be run using [`CallEval::run`].
#[derive(Clone)]
pub struct CallEval {
    callee_stack: Stack,
    head_span: Span,
    callee_span: Span,
    arg_index: usize,
    /// The variables of the flags bound so far.
    named_args: Vec<VarId>,
    rest_args: Vec<Value>,
    /// After a list/null rest spread, further positionals go to rest (matches IR `always_spread`).
    always_spread: bool,
    eval: EvalBlockWithEarlyReturnFn,
}

impl CallEval {
    /// Create a new [`CallEval`] context
    pub fn new(
        callee_stack: Stack,
        call_head: Span,
        callee_span: Span,
        eval: EvalBlockWithEarlyReturnFn,
    ) -> Self {
        Self {
            callee_stack,
            head_span: call_head,
            callee_span,
            arg_index: 0,
            named_args: Vec::new(),
            rest_args: Vec::new(),
            always_spread: false,
            eval,
        }
    }

    /// Add a positional argument to the call stack.
    ///
    /// Returns an error if the given `value` does not match the type of
    /// the argument according to the signature (see [`CallEval::new`]).
    pub fn add_positional(
        &mut self,
        signature: &Signature,
        value: Cow<Value>,
    ) -> Result<&mut Self, ShellError> {
        // An `exit` that arrives as a stream item ends the script. Nothing can catch it, and code
        // that ran with it as input could simply ignore it.
        if let Value::Error { error, .. } = value.as_ref()
            && error.is_exit()
        {
            self.reset_arguments();
            return Err((**error).clone());
        }

        // After a rest spread, further positionals always go to rest (IR parity).
        if self.always_spread {
            return self.push_rest(signature, value);
        }

        let maybe_param = match self
            .arg_index
            .checked_sub(signature.required_positional.len())
        {
            // arg_index < required_len
            None => signature.required_positional.get(self.arg_index),
            // required_len <= arg_index < (required_len + optional_len)
            Some(opt_idx) if opt_idx < signature.optional_positional.len() => {
                signature.optional_positional.get(opt_idx)
            }
            // (required_len + optional_len) <= arg_index
            _ => None,
        };

        if let Some(param) = maybe_param {
            let param_type = param.shape.to_type();
            if !value.is_subtype_of(&param_type) {
                self.reset_arguments();
                return Err(ShellError::CantConvert {
                    to_type: param_type.to_string(),
                    from_type: value.get_type().to_string(),
                    span: value.span(),
                    help: value.explain_type_mismatch(&param_type),
                });
            }

            let var_id = param
                .var_id
                .expect("internal error: all custom parameters must have var_ids");
            self.callee_stack.add_var(var_id, value.into_owned());
            self.arg_index += 1;
            Ok(self)
        } else {
            self.push_rest(signature, value)
        }
    }

    fn push_rest(
        &mut self,
        signature: &Signature,
        value: Cow<Value>,
    ) -> Result<&mut Self, ShellError> {
        let Some(rest_positional) = &signature.rest_positional else {
            // We do not consider it an error if more arguments
            // are added than the closure takes. This makes it possible
            // to omit any unused arguments in the closure definition.
            return Ok(self);
        };

        let param_type = rest_positional.shape.to_type();
        if !value.is_subtype_of(&param_type) {
            self.reset_arguments();
            return Err(ShellError::CantConvert {
                to_type: param_type.to_string(),
                from_type: value.get_type().to_string(),
                span: value.span(),
                help: value.explain_type_mismatch(&param_type),
            });
        }

        self.rest_args.push(value.into_owned());
        Ok(self)
    }

    /// Forget the positional arguments bound so far, so the next call made with
    /// this [`CallEval`] binds its arguments from the first parameter again.
    ///
    /// This runs after every call, and also when an argument can't be bound,
    /// because the caller can't run that call but may go on to make another one
    /// (for example a command that maps a closure over a stream and keeps going
    /// after an item fails).
    fn reset_arguments(&mut self) {
        self.arg_index = 0;
        self.rest_args.clear();
    }

    /// Spread a list into rest (IR parity), and send further positionals to rest too. A `null`
    /// spread is an empty list here. Errors if required positionals remain unbound.
    fn add_list_spread(
        &mut self,
        signature: &Signature,
        vals: Vec<Value>,
        spread_span: Span,
    ) -> Result<&mut Self, ShellError> {
        if self.arg_index < signature.required_positional.len() && !self.always_spread {
            return Err(list_spread_before_required_error(spread_span));
        }
        for v in vals {
            self.push_rest(signature, Cow::Owned(v))?;
        }
        self.always_spread = true;
        Ok(self)
    }

    /// Spread a value into the call, as `...value` does in a call to a custom command.
    ///
    /// A record sets named flags, a list fills the rest parameter, and `null` sends further
    /// positional arguments to the rest parameter (IR parity).
    pub fn add_spread(
        &mut self,
        signature: &Signature,
        value: Value,
        spread_span: Span,
    ) -> Result<&mut Self, ShellError> {
        match value {
            Value::Record { val, .. } => {
                for arg in expand_flag_record(signature, val.into_owned(), spread_span)? {
                    match arg {
                        EngineArgument::Flag { data, name, .. } => {
                            let long = std::str::from_utf8(&data[name]).unwrap_or_default();
                            self.add_named(signature, long, None, None)?;
                        }
                        EngineArgument::Named {
                            data, name, val, ..
                        } => {
                            let long = std::str::from_utf8(&data[name]).unwrap_or_default();
                            self.add_named(signature, long, None, Some(Cow::Owned(val)))?;
                        }
                        _ => {}
                    }
                }
                Ok(self)
            }
            Value::List { .. } if !can_rest_spread(signature) => {
                Err(list_spread_without_rest_error(spread_span))
            }
            Value::List { vals, .. } => {
                self.add_list_spread(signature, vals.into_owned(), spread_span)
            }
            Value::Nothing { .. } => self.add_list_spread(signature, Vec::new(), spread_span),
            Value::Error { error, .. } => Err(*error),
            other => Err(ShellError::CannotSpreadAsList { span: other.span() }),
        }
    }

    /// Add a named parameter to the call stack.
    ///
    /// The flag is found by `long`, or by `short` when `long` is empty, as for a flag that only
    /// has a short name. Returns an error for a flag without a variable to bind, like the
    /// built-in `--help`.
    pub fn add_named(
        &mut self,
        signature: &Signature,
        long: &str,
        short: Option<String>,
        value: Option<Cow<Value>>,
    ) -> Result<&mut Self, ShellError> {
        // Another flag may declare the same short, so a short only names a flag without a long name.
        let short = short.filter(|_| long.is_empty()).unwrap_or_default();
        if let Some(named) = find_signature_flag(signature, long.as_bytes(), short.as_bytes()) {
            let var_id = expect_named_var_id(named, self.head_span)?;

            let value = value
                .or_else(|| named.default_value.as_ref().map(Cow::Borrowed))
                .unwrap_or_else(|| Cow::Owned(Value::bool(true, self.head_span)));

            self.callee_stack.add_var(var_id, value.into_owned());
            self.named_args.push(var_id);
        }

        Ok(self)
    }

    /// Sets the environment variables for the call.
    pub fn with_env(
        &mut self,
        env_vars: &[Arc<EnvVars>],
        env_hidden: &Arc<HashMap<String, HashSet<EnvName>>>,
    ) -> &mut Self {
        self.callee_stack.with_env(env_vars, env_hidden);
        self
    }

    /// Sets whether to enable debugging when evaluating the closure.
    pub fn debug(&mut self, debug: bool) -> &mut Self {
        if debug {
            self.eval = eval_block_with_early_return::<WithDebug>
        } else {
            self.eval = eval_block_with_early_return::<WithoutDebug>
        };
        self
    }

    /// Run the given block.
    pub fn run(
        &mut self,
        engine_state: &EngineState,
        block: &Block,
        input: PipelineData,
    ) -> Result<PipelineData, ShellError> {
        // Reset even when a parameter is missing, so the error doesn't leak into the next call.
        let finalized = self.finalize_arguments(&block.signature);
        self.reset_arguments();
        finalized?;
        (self.eval)(engine_state, &mut self.callee_stack, block, input).map(|p| p.body)
    }

    /// Export the modified environment from callee to the caller.
    pub fn redirect_env(&self, engine_state: &EngineState, stack: &mut Stack) {
        redirect_env(engine_state, stack, &self.callee_stack);
    }

    /// Add default and rest values to the stack, raise error on
    /// missing parameters.
    fn finalize_arguments(&mut self, signature: &Signature) -> Result<(), ShellError> {
        let remaining_positionals = signature
            .required_positional
            .iter()
            .map(|p| (p, true))
            .chain(signature.optional_positional.iter().map(|p| (p, false)))
            // skip positional args added with add_positional
            .skip(self.arg_index);

        for (param, required) in remaining_positionals {
            let var_id = param
                .var_id
                .expect("internal error: all custom parameters must have var_ids");

            let maybe_value = param
                .default_value
                .clone()
                .or((!required).then_some(Value::nothing(self.callee_span)));

            if let Some(value) = maybe_value {
                self.callee_stack.add_var(var_id, value);
            } else {
                return Err(ShellError::MissingParameter {
                    param_name: param.name.to_string(),
                    span: self.callee_span,
                });
            }
        }

        if let Some(rest_positional) = &signature.rest_positional {
            let span = self
                .rest_args
                .first()
                .map(|x| x.span())
                .unwrap_or(self.callee_span);

            self.callee_stack.add_var(
                rest_positional
                    .var_id
                    .expect("Internal error: rest positional parameter lackes var_id"),
                Value::list(self.rest_args.to_owned(), span),
            );
        }

        let remaining_flags = signature
            .named
            .iter()
            // Ignore named arguments without var_id.
            // There is some code in nu_cli::completions that relies on this behavior.
            .filter_map(|flag| Some((flag.var_id?, flag)))
            // Skip provided flags
            .filter(|(var_id, _)| !self.named_args.contains(var_id));

        for (var_id, flag) in remaining_flags {
            if flag.arg.is_none() {
                self.callee_stack
                    .add_var(var_id, Value::bool(false, self.head_span));
            } else {
                let value = flag
                    .default_value
                    .clone()
                    .unwrap_or(Value::nothing(self.head_span));
                self.callee_stack.add_var(var_id, value);
            }
        }

        Ok(())
    }
}

/// Redirect the environment from callee to the caller.
pub fn redirect_env(engine_state: &EngineState, caller_stack: &mut Stack, callee_stack: &Stack) {
    // Grab all environment variables from the callee
    let caller_env_vars = caller_stack.get_env_var_names(engine_state);

    // remove env vars that are present in the caller but not in the callee
    // (the callee hid them)
    for var in caller_env_vars.iter() {
        if !callee_stack.has_env_var(engine_state, var) {
            caller_stack.hide_env_var(engine_state, var);
        }
    }

    // add new env vars from callee to caller
    for (var, value) in callee_stack.get_stack_env_vars() {
        caller_stack.add_env_var(var, value);
    }
    caller_stack.last_exit_code_written |= callee_stack.last_exit_code_written;

    // set config to callee config, to capture any updates to that
    caller_stack.config.clone_from(&callee_stack.config);
}

pub fn eval_block<D: DebugContext>(
    engine_state: &EngineState,
    stack: &mut Stack,
    block: &Block,
    input: PipelineData,
) -> Result<PipelineExecutionData, ShellError> {
    let result = eval_ir_block::<D>(engine_state, stack, block, input);
    if let Err(err) = &result {
        stack.set_last_error(err);
    }
    result
}

/// Evaluate a block as an early return boundary.
///
/// A `return` is meant to end the command or closure it appears in and go no further. The
/// "boundary" is the point where such a `return` stops propagating outward and becomes the
/// block's normal result, instead of escaping to whatever called the command. This function is
/// that point: an early `return` inside the block produces the block's result here, exactly like
/// a value in tail position.
///
/// Concretely, [`eval_block`] runs the block and sets
/// [`early_return`](PipelineExecutionData::early_return) to mark a result that came from a
/// `return`; this function clears that flag, which is what "absorbs" the `return` so callers see
/// an ordinary result.
///
/// This is used for blocks that `return` should not escape from, such as custom command bodies
/// and closures: clearing the flag keeps an early `return` from leaking into the calling block.
/// In contrast, [`eval_block`] leaves the flag intact, so its one consumer (top-level file
/// evaluation) can see a top-level `return` and skip running `main`.
pub fn eval_block_with_early_return<D: DebugContext>(
    engine_state: &EngineState,
    stack: &mut Stack,
    block: &Block,
    input: PipelineData,
) -> Result<PipelineExecutionData, ShellError> {
    let mut result = eval_block::<D>(engine_state, stack, block, input)?;
    result.early_return = false;
    Ok(result)
}

pub fn eval_variable(
    engine_state: &EngineState,
    stack: &Stack,
    var_id: VarId,
    span: Span,
) -> Result<Value, ShellError> {
    match var_id {
        // $nu
        nu_protocol::NU_VARIABLE_ID => {
            if let Some(val) = engine_state.get_constant(var_id) {
                Ok(val.clone())
            } else {
                Err(ShellError::VariableNotFoundAtRuntime { span })
            }
        }
        // $env
        ENV_VARIABLE_ID => {
            let env_vars = stack.get_env_vars(engine_state);
            let env_columns = env_vars.keys();
            let env_values = env_vars.values();

            let mut pairs = env_columns
                .map(|x| x.to_string())
                .zip(env_values.cloned())
                .collect::<Vec<(String, Value)>>();

            pairs.sort_by(|a, b| a.0.cmp(&b.0));

            Ok(Value::record(pairs.into_iter().collect(), span))
        }
        // interactive last-result (e.g. `$ans`)
        // Truncation warning is deferred until after print so data is visible first.
        id if id == nu_protocol::LAST_VARIABLE_ID => {
            stack.defer_last_result_truncation_warning();
            stack.get_var(var_id, span)
        }
        var_id => stack.get_var(var_id, span),
    }
}
