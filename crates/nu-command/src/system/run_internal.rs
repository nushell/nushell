use nu_engine::{CallExt, command_prelude::*, eval_ir_call, find_builtin_decl};
use nu_protocol::ir;

/// Internal command used by the `%($cmd)`/`%$cmd` dynamic builtin dispatch syntax.
///
/// The `%` sigil statically resolves a builtin at parse time. When the head is a runtime
/// expression (`%($cmd)` or `%$cmd`), the parser defers resolution and the IR compiler
/// rewrites the call as `run-internal <head-expr> ...args`. This command then looks up the
/// target builtin at runtime and enforces that it is a `CommandType::Builtin`.
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
        let name: String = call.req(engine_state, stack, 0)?;

        let decl_id = find_builtin_decl(engine_state, &name)
            .ok_or(ShellError::CommandNotFound { span: head })?;

        // Preserve spread markers here so `%($cmd) ...$args` forwards the same argument shape the
        // target builtin would receive in a direct call.
        let rest_args: Vec<(Value, bool)> = call.rest_preserving_spreads(engine_state, stack, 1)?;

        // Build an IR call frame for the target builtin, preserving spread arguments.
        let mut builder = ir::Call::build(decl_id, head);
        for (val, is_spread) in rest_args {
            if is_spread {
                // Skip empty spreads so builtins with no-argument defaults (e.g. `ls`
                // listing the cwd when called with no path) are not confused by an
                // empty `...$args` forwarded as an empty list.
                match val {
                    Value::List { ref vals, .. } if vals.is_empty() => continue,
                    Value::Nothing { .. } => continue,
                    _ => {
                        builder.add_spread(stack, head, val);
                    }
                }
            } else {
                builder.add_positional(stack, head, val);
            }
        }

        // Run it like a direct call, so its input is type checked and record spreads become flags.
        let call = builder.finish();
        let result = eval_ir_call(engine_state, stack, &call, input);
        call.leave(stack);
        result
    }
}
