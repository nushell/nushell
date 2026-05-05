use nu_engine::{CallExt, command_prelude::*};
use nu_protocol::{DeclId, engine::CommandType, ir};

/// Find a built-in declaration by name, ignoring normal scope visibility.
///
/// This intentionally mirrors static `%name` behavior in the parser, where `%` means
/// "resolve as built-in" even if a custom declaration shadows the same name.
fn find_builtin_decl(engine_state: &EngineState, name: &[u8]) -> Option<DeclId> {
    for idx in (0..engine_state.num_decls()).rev() {
        let decl_id = DeclId::new(idx);
        let decl = engine_state.get_decl(decl_id);
        if decl.command_type() == CommandType::Builtin && decl.name().as_bytes() == name {
            return Some(decl_id);
        }
    }

    None
}

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

        let decl_id = find_builtin_decl(engine_state, name.as_bytes())
            .ok_or(ShellError::CommandNotFound { span: head })?;

        let decl = engine_state.get_decl(decl_id);
        // Preserve spread markers here so `%($cmd) ...$args` forwards the same argument shape the
        // target builtin would receive in a direct call.
        let rest_args: Vec<(Value, bool)> = call.rest_preserving_spreads(engine_state, stack, 1)?;

        // Build an IR call frame for the target builtin, preserving spread arguments.
        let mut builder = ir::Call::build(decl_id, head);
        for (val, is_spread) in rest_args {
            if is_spread {
                builder.add_spread(stack, head, val);
            } else {
                builder.add_positional(stack, head, val);
            }
        }

        // Ensure temporary IR arguments are always cleaned up after dispatch.
        builder.with(stack, |stack, engine_call| {
            decl.run(engine_state, stack, engine_call, input)
        })
    }
}
