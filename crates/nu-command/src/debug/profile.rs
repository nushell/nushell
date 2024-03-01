use nu_engine::{eval_block_with_early_return, CallExt};
use nu_protocol::ast::Call;
use nu_protocol::debugger::{Profiler, WithDebug};
use nu_protocol::engine::{Closure, Command, EngineState, Stack};
use nu_protocol::{
    Category, Example, IntoPipelineData, PipelineData, ShellError, Signature, SyntaxShape, Type,
};

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
            .switch("spans", "Collect spans", Some('s'))
            .switch("expand-source", "Collect full source fragments", Some('e'))
            .switch(
                "values",
                "Collect pipeline output values of pipeline elements",
                Some('v'),
            )
            .switch("expr", "Collect expression types", Some('x'))
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
        let collect_spans = call.has_flag(engine_state, caller_stack, "spans")?;
        let collect_expanded_source =
            call.has_flag(engine_state, caller_stack, "expanded-source")?;
        let collect_values = call.has_flag(engine_state, caller_stack, "values")?;
        let collect_exprs = call.has_flag(engine_state, caller_stack, "expr")?;
        let max_depth = call
            .get_flag(engine_state, caller_stack, "max-depth")?
            .unwrap_or(default_max_depth);

        let profiler = Profiler::new(
            max_depth,
            collect_spans,
            true,
            collect_expanded_source,
            collect_values,
            collect_exprs,
            call.span(),
        );

        let lock_err = {
            |_| ShellError::GenericError {
                error: "Profiler Error".to_string(),
                msg: "could not lock debugger, poisoned mutex".to_string(),
                span: Some(call.head),
                help: None,
                inner: vec![],
            }
        };

        engine_state
            .activate_debugger(Box::new(profiler))
            .map_err(lock_err)?;

        let result = eval_block_with_early_return::<WithDebug>(
            engine_state,
            &mut callee_stack,
            block,
            input,
            call.redirect_stdout,
            call.redirect_stdout,
        );

        // TODO: See eval_source()
        match result {
            Ok(pipeline_data) => {
                let _ = pipeline_data.into_value(call.span());
                // pipeline_data.print(engine_state, caller_stack, true, false)
            }
            Err(_e) => (), // TODO: Report error
        }

        // TODO: Make report() Profiler-only
        let res = engine_state
            .debugger
            .lock()
            .map_err(lock_err)?
            .report(engine_state, call.span());

        engine_state.deactivate_debugger().map_err(lock_err)?;

        res.map(|val| val.into_pipeline_data())
    }

    fn examples(&self) -> Vec<Example> {
        vec![Example {
            description: "Profile config evaluation time",
            example: "debug profile { source $nu.config-path }",
            result: None,
        }]
    }
}
