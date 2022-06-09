use nu_engine::{eval_block, CallExt};
use nu_protocol::engine::{CaptureBlock, Command, EngineState, Stack};
use nu_protocol::{ast::Call, Span};
use nu_protocol::{
    Category, Example, IntoInterruptiblePipelineData, PipelineData, Signature, SyntaxShape, Value,
};

#[derive(Clone)]
pub struct SkipIf;

impl Command for SkipIf {
    fn name(&self) -> &str {
        "skip if"
    }

    fn usage(&self) -> &str {
        "Skip elements of the input where a predicate is true."
    }

    fn signature(&self) -> nu_protocol::Signature {
        Signature::build(self.name())
            .required(
                "predicate",
                SyntaxShape::RowCondition,
                "the predicate that skipped element must match",
            )
            .category(Category::Filters)
    }

    fn run(
        &self,
        engine_state: &EngineState,
        stack: &mut Stack,
        call: &Call,
        input: PipelineData,
    ) -> Result<nu_protocol::PipelineData, nu_protocol::ShellError> {
        let span = call.head;

        let metadata = input.metadata();

        let block: CaptureBlock = call.req(engine_state, stack, 0)?;
        let mut stack = stack.captures_to_stack(&block.captures);
        let block = engine_state.get_block(block.block_id).clone();

        let ctrlc = engine_state.ctrlc.clone();
        let engine_state = engine_state.clone();

        let redirect_stdout = call.redirect_stdout;
        let redirect_stderr = call.redirect_stderr;

        Ok(input
            .into_iter()
            .filter_map(move |value| {
                if let Some(var) = block.signature.get_positional(0) {
                    if let Some(var_id) = &var.var_id {
                        stack.add_var(*var_id, value.clone());
                    }
                }
                let result = eval_block(
                    &engine_state,
                    &mut stack,
                    &block,
                    PipelineData::new(span),
                    redirect_stdout,
                    redirect_stderr,
                );

                match result {
                    Ok(result) => {
                        let result = result.into_value(span);
                        if result.is_true() {
                            None
                        } else {
                            Some(value)
                        }
                    }
                    Err(err) => Some(Value::Error { error: err }),
                }
            })
            .into_pipeline_data(ctrlc))
        .map(|x| x.set_metadata(metadata))
    }

    fn examples(&self) -> Vec<Example> {
        vec![Example {
            description: "Skip if the element is negative",
            example: "echo [-2 0 2 -1] | skip if $it < 0",
            result: Some(Value::List {
                vals: vec![Value::test_int(0), Value::test_int(2)],
                span: Span::test_data(),
            }),
        }]
    }
}
