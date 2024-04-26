use nu_engine::{command_prelude::*, ClosureEvalOnce};
use nu_protocol::{debugger::Profiler, engine::Closure};

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
            .switch("spans", "Collect spans of profiled elements", Some('s'))
            .switch(
                "expand-source",
                "Collect full source fragments of profiled elements",
                Some('e'),
            )
            .switch(
                "values",
                "Collect pipeline element output values",
                Some('v'),
            )
            .switch("expr", "Collect expression types", Some('x'))
            .named(
                "max-depth",
                SyntaxShape::Int,
                "How many blocks/closures deep to step into (default 2)",
                Some('m'),
            )
            .input_output_types(vec![(Type::Any, Type::table())])
            .category(Category::Debug)
    }

    fn usage(&self) -> &str {
        "Profile pipeline elements in a closure."
    }

    fn extra_usage(&self) -> &str {
        r#"The profiler profiles every evaluated pipeline element inside a closure, stepping into all
commands calls and other blocks/closures.

The output can be heavily customized. By default, the following columns are included:
- depth       : Depth of the pipeline element. Each entered block adds one level of depth. How many
                blocks deep to step into is controlled with the --max-depth option.
- id          : ID of the pipeline element
- parent_id   : ID of the parent element
- source      : Source code of the pipeline element. If the element has multiple lines, only the
                first line is used and `...` is appended to the end. Full source code can be shown
                with the  --expand-source flag.
- duration_ms : How long it took to run the pipeline element in milliseconds.
- (optional) span   : Span of the element. Can be viewed via the `view span` command. Enabled with
                      the --spans flag.
- (optional) expr   : The type of expression of the pipeline element. Enabled with the --expr flag.
- (optional) output : The output value of the pipeline element. Enabled with the --values flag.

To illustrate the depth and IDs, consider `debug profile { if true { echo 'spam' } }`. There are
three pipeline elements:

depth  id  parent_id
    0   0          0  debug profile { do { if true { 'spam' } } }
    1   1          0  if true { 'spam' }
    2   2          1  'spam'

Each block entered increments depth by 1 and each block left decrements it by one. This way you can
control the profiling granularity. Passing --max-depth=1 to the above would stop at
`if true { 'spam' }`. The id is used to identify each element. The parent_id tells you that 'spam'
was spawned from `if true { 'spam' }` which was spawned from the root `debug profile { ... }`.

Note: In some cases, the ordering of piepeline elements might not be intuitive. For example,
`[ a bb cc ] | each { $in | str length }` involves some implicit collects and lazy evaluation
confusing the id/parent_id hierarchy. The --expr flag is helpful for investigating these issues."#
    }

    fn run(
        &self,
        engine_state: &EngineState,
        stack: &mut Stack,
        call: &Call,
        input: PipelineData,
    ) -> Result<PipelineData, ShellError> {
        let closure: Closure = call.req(engine_state, stack, 0)?;
        let collect_spans = call.has_flag(engine_state, stack, "spans")?;
        let collect_expanded_source = call.has_flag(engine_state, stack, "expanded-source")?;
        let collect_values = call.has_flag(engine_state, stack, "values")?;
        let collect_exprs = call.has_flag(engine_state, stack, "expr")?;
        let max_depth = call
            .get_flag(engine_state, stack, "max-depth")?
            .unwrap_or(2);

        let profiler = Profiler::new(
            max_depth,
            collect_spans,
            true,
            collect_expanded_source,
            collect_values,
            collect_exprs,
            call.span(),
        );

        let lock_err = |_| ShellError::GenericError {
            error: "Profiler Error".to_string(),
            msg: "could not lock debugger, poisoned mutex".to_string(),
            span: Some(call.head),
            help: None,
            inner: vec![],
        };

        engine_state
            .activate_debugger(Box::new(profiler))
            .map_err(lock_err)?;

        let result = ClosureEvalOnce::new(engine_state, stack, closure).run_with_input(input);

        // TODO: See eval_source()
        match result {
            Ok(pipeline_data) => {
                let _ = pipeline_data.into_value(call.span());
                // pipeline_data.print(engine_state, caller_stack, true, false)
            }
            Err(_e) => (), // TODO: Report error
        }

        Ok(engine_state
            .deactivate_debugger()
            .map_err(lock_err)?
            .report(engine_state, call.span())?
            .into_pipeline_data())
    }

    fn examples(&self) -> Vec<Example> {
        vec![
            Example {
                description: "Profile config evaluation",
                example: "debug profile { source $nu.config-path }",
                result: None,
            },
            Example {
                description: "Profile config evaluation with more granularity",
                example: "debug profile { source $nu.config-path } --max-depth 4",
                result: None,
            },
        ]
    }
}
