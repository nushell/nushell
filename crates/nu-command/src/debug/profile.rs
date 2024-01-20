use nu_engine::{eval_block_with_early_return, CallExt};
use nu_protocol::ast::Call;
use nu_protocol::debugger::{Profiler, WithDebug, WithoutDebug};
use nu_protocol::engine::{Closure, Command, EngineState, Stack};
use nu_protocol::{
    record, Category, Example, IntoPipelineData, LazyRecord, PipelineData, Record, ShellError,
    Signature, Span, SyntaxShape, Type, Value,
};
use std::sync::{Arc, Mutex};

#[derive(Clone)]
pub struct DebugProfile;

impl Command for DebugProfile {
    fn name(&self) -> &str {
        "debug profile"
    }

    fn signature(&self) -> nu_protocol::Signature {
        Signature::build("debug profile")
            .required(
                "closure",
                SyntaxShape::Closure(None),
                "The closure to profile.",
            )
            .input_output_types(vec![(Type::Any, Type::Table(vec![]))])
            .category(Category::Debug)
    }

    fn usage(&self) -> &str {
        "Profile a closure."
    }

    fn extra_usage(&self) -> &str {
        ""
    }

    fn run(
        &self,
        engine_state: &EngineState,
        caller_stack: &mut Stack,
        call: &Call,
        input: PipelineData,
    ) -> Result<PipelineData, ShellError> {
        let closure: Closure = call.req(engine_state, caller_stack, 0)?;
        let mut callee_stack = caller_stack.captures_to_stack(closure.captures);
        let block = engine_state.get_block(closure.block_id);

        let profiler = Arc::new(Mutex::new(Profiler::default()));

        let result = eval_block_with_early_return(
            engine_state,
            &mut callee_stack,
            block,
            input,
            call.redirect_stdout,
            call.redirect_stdout,
            // DEBUG TODO
            WithDebug,
            &Some(profiler),
        );

        Ok(PipelineData::empty())
    }

    fn examples(&self) -> Vec<Example> {
        vec![Example {
            description: "Profile config evaluation time",
            example: "debug profile { source $nu.config-path }",
            result: None,
        }]
    }
}
