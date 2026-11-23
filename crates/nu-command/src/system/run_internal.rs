use crate::misc::deferred_arguments::{
    ArgumentTarget, DeferredArgument, bind_deferred_arguments, call_arguments, deferred_arguments,
    split_off_callee_arguments,
};
use nu_engine::{
    DEFERRED_ARGUMENTS_PARSER_INFO, command_prelude::*, eval_ir_call, find_builtin_decl,
    get_full_help,
};
use nu_protocol::{
    Flag, FromValue, ParseError,
    engine::{self, named_flags::check_switch_value},
    ir,
};

/// Internal command used by the `%($cmd)`/`%$cmd` dynamic builtin dispatch syntax.
///
/// The `%` sigil statically resolves a builtin at parse time. When the head is a runtime
/// expression (`%($cmd)` or `%$cmd`), the parser defers resolution and the IR compiler
/// rewrites the call as `run-internal <head-expr> ...args`. This command then looks up the
/// target builtin at runtime, enforces that it is a `CommandType::Builtin`, and binds the
/// arguments to its signature, as `run` binds a script's arguments to its `main`. A direct
/// `run-internal` call passes its arguments on as they are.
#[derive(Clone)]
pub struct RunInternal;

impl Command for RunInternal {
    fn name(&self) -> &str {
        "run-internal"
    }

    fn description(&self) -> &str {
        "Run a built-in command by name. Used internally by `%($cmd)` dynamic dispatch."
    }

    fn signature(&self) -> Signature {
        Signature::build("run-internal")
            .input_output_types(vec![(Type::Any, Type::Any)])
            .required(
                "name",
                SyntaxShape::String,
                "The name of the built-in command to run.",
            )
            // For `%$cmd`, the parser passes each spread and `--name=value` after the name
            // wrapped in a one-item list. The evaluator checks them against this signature, which
            // needs a rest parameter to let the spreads through.
            .rest(
                "args",
                SyntaxShape::Any,
                "Arguments to pass to the built-in command.",
            )
    }

    fn run(
        &self,
        engine_state: &EngineState,
        stack: &mut Stack,
        call: &Call,
        input: PipelineData,
    ) -> Result<PipelineData, ShellError> {
        let head = call.head;
        // Only the compiler's lowering of `%$cmd` adds this parser info, so a call without it is a
        // direct `run-internal`.
        let dynamic_dispatch = call
            .get_parser_info(stack, DEFERRED_ARGUMENTS_PARSER_INFO)
            .is_some();

        let mut args = call_arguments(stack, call);
        let builtin_args = split_off_callee_arguments(&mut args);
        // The split leaves the builtin's name last.
        let name = match args.pop() {
            Some(engine::Argument::Positional { val, .. }) => String::from_value(val)?,
            _ => {
                return Err(ShellError::MissingParameter {
                    param_name: "name".into(),
                    span: head,
                });
            }
        };

        let decl_id = find_builtin_decl(engine_state, &name)
            .ok_or(ShellError::CommandNotFound { span: head })?;
        let signature = engine_state.get_signature_cached(decl_id);

        let mut builtin_arguments = BuiltinArguments {
            builder: ir::Call::build(decl_id, head),
            stack,
            positionals: 0,
        };
        // Whether the arguments ask for the builtin's help instead of running it.
        let help = if dynamic_dispatch {
            deferred_arguments(builtin_args).and_then(|args| {
                let end = args.last().map_or(head, DeferredArgument::span).end;
                let help = bind_deferred_arguments(
                    engine_state,
                    &signature,
                    args,
                    &mut builtin_arguments,
                )?;
                // As in a direct call, a required positional argument that's left out is an error
                // unless the arguments ask for help. It points at the end of the call, as the
                // parser's error does.
                match signature
                    .required_positional
                    .get(builtin_arguments.positionals)
                {
                    Some(missing) if !help => {
                        Err(ShellError::from_diagnostic(&ParseError::MissingPositional(
                            missing.name.clone(),
                            Span::new(end, end),
                            signature.call_signature(),
                        )))
                    }
                    _ => Ok(help),
                }
            })
        } else {
            // A direct call passes its positional and spread arguments on as values.
            builtin_args
                .into_iter()
                .try_for_each(|arg| match arg {
                    engine::Argument::Positional { val, span } => {
                        builtin_arguments.add_positional(&signature, val, span)
                    }
                    engine::Argument::Spread { vals, span } => {
                        builtin_arguments.add_spread(&signature, vals, span)
                    }
                    // `run-internal` has no flags of its own to pass on.
                    _ => Ok(()),
                })
                .map(|()| false)
        };
        let BuiltinArguments { builder, stack, .. } = builtin_arguments;

        // Run it like a direct call, so its input is type checked and record spreads become flags.
        if matches!(help, Ok(false)) {
            return eval_ir_call(engine_state, stack, builder, input);
        }

        // The builtin won't run, so nothing else takes its arguments off the stack.
        builder.finish().leave(stack);
        help.map(|_| {
            let help = get_full_help(engine_state.get_decl(decl_id), engine_state, stack, head);
            Value::string(help, head).into_pipeline_data()
        })
    }
}

/// Builds the call to the builtin that `run-internal` runs.
///
/// The values go to the builtin as they are, and it converts each one to the type it takes, as it
/// does for the values of a direct call. Only a switch's value is checked here, because a builtin
/// reads any value but `false` as the switch being on.
struct BuiltinArguments<'a> {
    builder: ir::CallBuilder,
    stack: &'a mut Stack,
    /// The number of positional arguments added so far.
    positionals: usize,
}

impl ArgumentTarget for BuiltinArguments<'_> {
    fn add_positional(
        &mut self,
        _: &Signature,
        value: Value,
        span: Span,
    ) -> Result<(), ShellError> {
        self.builder.add_positional(self.stack, span, value);
        self.positionals += 1;
        Ok(())
    }

    fn add_spread(&mut self, _: &Signature, value: Value, span: Span) -> Result<(), ShellError> {
        // Skip empty spreads so builtins with no-argument defaults (e.g. `ls` listing the cwd
        // when called with no path) are not confused by an empty `...$args`.
        let empty = match &value {
            Value::List { vals, .. } => vals.is_empty(),
            Value::Nothing { .. } => true,
            _ => false,
        };
        if !empty {
            self.builder.add_spread(self.stack, span, value);
        }
        Ok(())
    }

    fn add_named(
        &mut self,
        _: &Signature,
        flag: &Flag,
        span: Span,
        value: Option<(Value, Span)>,
    ) -> Result<(), ShellError> {
        if let Some((value, value_span)) = &value {
            check_switch_value(flag, value, *value_span)?;
        }
        let short = flag.short.map(String::from).unwrap_or_default();
        match value {
            Some((value, _)) => self
                .builder
                .add_named(self.stack, &flag.long, short, span, value),
            None => self.builder.add_flag(self.stack, &flag.long, short, span),
        };
        Ok(())
    }
}
