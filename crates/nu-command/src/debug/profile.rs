use nu_engine::{ClosureEvalOnce, command_prelude::*};
use nu_protocol::{
    debugger::{DurationMode, Profiler, ProfilerOptions},
    engine::Closure,
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
            .switch("lines", "Collect line numbers", Some('l'))
            .switch(
                "duration-values",
                "Report instruction duration as duration values rather than milliseconds",
                Some('d'),
            )
            .named(
                "max-depth",
                SyntaxShape::Int,
                "How many blocks/closures deep to step into (default 2)",
                Some('m'),
            )
            .input_output_types(vec![(Type::Any, Type::table())])
            .category(Category::Debug)
    }

    fn description(&self) -> &str {
        "Profile pipeline elements in a closure."
    }

    fn extra_description(&self) -> &str {
        r#"The profiler profiles every evaluated instruction inside a closure, stepping into all
commands calls and other blocks/closures.

The output can be heavily customized. By default, the following columns are included:
- depth       : Depth of the instruction. Each entered block adds one level of depth. How many
                blocks deep to step into is controlled with the --max-depth option.
- id          : ID of the instruction
- parent_id   : ID of the instruction that created the parent scope
- source      : Source code that generated the instruction. If the source code has multiple lines,
                only the first line is used and `...` is appended to the end. Full source code can
                be shown with the --expand-source flag.
- pc          : The index of the instruction within the block.
- instruction : The pretty printed instruction being evaluated.
- duration    : How long it took to run the instruction.
- (optional) span        : Span associated with the instruction. Can be viewed via the `view span`
                           command. Enabled with the --spans flag.
- (optional) output      : The output value of the instruction. Enabled with the --values flag.

To illustrate the depth and IDs, consider `debug profile { do { if true { echo 'spam' } } }`. A unique ID is generated each time an instruction is executed, and there are two levels of depth:

```
depth   id   parent_id                    source                     pc            instruction                
    0    0           0   debug profile { do { if true { 'spam' } } }  0   <start>                                   
    1    1           0   { if true { 'spam' } }                       0   load-literal    %1, closure(2164)  
    1    2           0   { if true { 'spam' } }                       1   push-positional %1                 
    1    3           0   { do { if true { 'spam' } } }                2   redirect-out    caller             
    1    4           0   { do { if true { 'spam' } } }                3   redirect-err    caller             
    1    5           0   do                                           4   call            decl 7 "do", %0    
    2    6           5   true                                         0   load-literal    %1, bool(true)     
    2    7           5   if                                           1   not             %1                 
    2    8           5   if                                           2   branch-if       %1, 5              
    2    9           5   'spam'                                       3   load-literal    %0, string("spam") 
    2   10           5   if                                           4   jump            6                  
    2   11           5   { if true { 'spam' } }                       6   return          %0                 
    1   12           0   { do { if true { 'spam' } } }                5   return          %0                 
```

Each block entered increments depth by 1 and each block left decrements it by one. This way you can
control the profiling granularity. Passing --max-depth=1 to the above would stop inside the `do`
at `if true { 'spam' }`. The id is used to identify each element. The parent_id tells you that the
instructions inside the block are being executed because of `do` (5), which in turn was spawned from
the root `debug profile { ... }`.

For a better understanding of how instructions map to source code, see the `view ir` command.

Note: In some cases, the ordering of pipeline elements might not be intuitive. For example,
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
        let collect_lines = call.has_flag(engine_state, stack, "lines")?;
        let duration_values = call.has_flag(engine_state, stack, "duration-values")?;
        let max_depth = call
            .get_flag(engine_state, stack, "max-depth")?
            .unwrap_or(2);

        let duration_mode = match duration_values {
            true => DurationMode::Value,
            false => DurationMode::Milliseconds,
        };
        let profiler = Profiler::new(
            ProfilerOptions {
                max_depth,
                collect_spans,
                collect_source: true,
                collect_expanded_source,
                collect_values,
                collect_exprs: false,
                collect_instructions: true,
                collect_lines,
                duration_mode,
            },
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

        // Return potential errors
        let pipeline_data = result?;

        // Collect the output
        let _ = pipeline_data.into_value(call.span());

        Ok(engine_state
            .deactivate_debugger()
            .map_err(lock_err)?
            .report(engine_state, call.span())?
            .into_pipeline_data())
    }

    fn examples(&self) -> Vec<Example<'_>> {
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
