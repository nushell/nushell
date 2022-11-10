use nu_engine::{eval_block, CallExt};
use nu_protocol::{
    ast::Call,
    engine::{Closure, Command, EngineState, Stack},
    Category, Example, IntoInterruptiblePipelineData, PipelineData, ShellError, Signature, Span,
    SyntaxShape, Type, Value,
};

#[derive(Clone)]
pub struct TakeUntil;

impl Command for TakeUntil {
    fn name(&self) -> &str {
        "take until"
    }

    fn signature(&self) -> Signature {
        Signature::build(self.name())
            .input_output_types(vec![
                (Type::Table(vec![]), Type::Table(vec![])),
                (
                    Type::List(Box::new(Type::Any)),
                    Type::List(Box::new(Type::Any)),
                ),
            ])
            .required(
                "predicate",
                SyntaxShape::RowCondition,
                "the predicate that element(s) must not match",
            )
            .category(Category::Filters)
    }

    fn usage(&self) -> &str {
        "Take elements of the input until a predicate is true."
    }

    fn examples(&self) -> Vec<Example> {
        vec![
            Example {
                description: "Take until the element is positive",
                example: "[-1 -2 9 1] | take until $it > 0",
                result: Some(Value::List {
                    vals: vec![Value::test_int(-1), Value::test_int(-2)],
                    span: Span::test_data(),
                }),
            },
            Example {
                description: "Take until the field value is positive",
                example: "[{a: -1} {a: -2} {a: 9} {a: 1}] | take until $it.a > 0",
                result: Some(Value::List {
                    vals: vec![
                        Value::test_record(vec!["a"], vec![Value::test_int(-1)]),
                        Value::test_record(vec!["a"], vec![Value::test_int(-2)]),
                    ],
                    span: Span::test_data(),
                }),
            },
        ]
    }

    fn run(
        &self,
        engine_state: &EngineState,
        stack: &mut Stack,
        call: &Call,
        input: PipelineData,
    ) -> Result<PipelineData, ShellError> {
        let span = call.head;

        let capture_block: Closure = call.req(engine_state, stack, 0)?;

        let block = engine_state.get_block(capture_block.block_id).clone();
        let var_id = block.signature.get_positional(0).and_then(|arg| arg.var_id);

        let mut stack = stack.captures_to_stack(&capture_block.captures);

        let ctrlc = engine_state.ctrlc.clone();
        let engine_state = engine_state.clone();

        let redirect_stdout = call.redirect_stdout;
        let redirect_stderr = call.redirect_stderr;

        Ok(input
            .into_iter()
            .take_while(move |value| {
                if let Some(var_id) = var_id {
                    stack.add_var(var_id, value.clone());
                }

                !eval_block(
                    &engine_state,
                    &mut stack,
                    &block,
                    PipelineData::new(span),
                    redirect_stdout,
                    redirect_stderr,
                )
                .map_or(false, |pipeline_data| {
                    pipeline_data.into_value(span).is_true()
                })
            })
            .into_pipeline_data(ctrlc))
    }
}

#[cfg(test)]
mod tests {
    use crate::TakeUntil;

    #[test]
    fn test_examples() {
        use crate::test_examples;

        test_examples(TakeUntil)
    }
}
