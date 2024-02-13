use nu_engine::{eval_block_with_early_return, CallExt};
use nu_protocol::ast::Call;
use nu_protocol::debugger::{Profiler, WithDebug};
use nu_protocol::engine::{Closure, Command, EngineState, Stack};
use nu_protocol::{
    Category, Example, IntoPipelineData, PipelineData, ShellError, Signature, SyntaxShape, Type,
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
            .switch("no-spans", "Do not collect spans", Some('n'))
            .switch("source", "Collect pipeline element sources", Some('s'))
            .switch(
                "values",
                "Collect pipeline element output values",
                Some('v'),
            )
            .named(
                "max-depth",
                SyntaxShape::Int,
                "How many blocks/closures deep to step into (default 2)",
                Some('m'),
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

        let default_max_depth = 2;
        let no_collect_spans = call.has_flag(engine_state, caller_stack, "no-spans")?;
        let collect_source = call.has_flag(engine_state, caller_stack, "source")?;
        let collect_values = call.has_flag(engine_state, caller_stack, "values")?;
        let max_depth = call
            .get_flag(engine_state, caller_stack, "max-depth")?
            .unwrap_or(default_max_depth);

        let profiler = Arc::new(Mutex::new(Profiler::new(
            max_depth,
            !no_collect_spans,
            collect_source,
            collect_values,
        )));

        callee_stack.with_debugger(profiler.clone());

        let result = eval_block_with_early_return(
            engine_state,
            &mut callee_stack,
            block,
            input,
            call.redirect_stdout,
            call.redirect_stdout,
            // DEBUG TODO
            WithDebug,
            &Some(profiler.clone()),
        );

        // TODO: See eval_source()
        match result {
            Ok(pipeline_data) => {
                let _ = pipeline_data.into_value(call.span());
                // pipeline_data.print(engine_state, caller_stack, true, false)
            }
            Err(e) => (), // TODO: Report error
        }

        // TODO unwrap
        let res = profiler.lock().unwrap().report(call.span());

        res.and_then(|val| Ok(val.into_pipeline_data()))
    }

    fn examples(&self) -> Vec<Example> {
        vec![Example {
            description: "Profile config evaluation time",
            example: "debug profile { source $nu.config-path }",
            result: None,
        }]
    }
}
