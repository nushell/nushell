use nu_engine::{eval_block, CallExt};
use nu_protocol::{
    ast::Call,
    engine::{Closure, Command, EngineState, Stack},
    record, Category, Example, IntoInterruptiblePipelineData, PipelineData, ShellError, Signature,
    SyntaxShape, Type, Value,
};

#[derive(Clone)]
pub struct TakeWhile;

impl Command for TakeWhile {
    fn name(&self) -> &str {
        "take while"
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
                SyntaxShape::Closure(Some(vec![SyntaxShape::Any, SyntaxShape::Int])),
                "the predicate that element(s) must match",
            )
            .category(Category::Filters)
    }

    fn usage(&self) -> &str {
        "Take elements of the input while a predicate is true."
    }

    fn examples(&self) -> Vec<Example> {
        vec![
            Example {
                description: "Take while the element is negative",
                example: "[-1 -2 9 1] | take while {|x| $x < 0 }",
                result: Some(Value::test_list(vec![
                    Value::test_int(-1),
                    Value::test_int(-2),
                ])),
            },
            Example {
                description: "Take while the element is negative using stored condition",
                example: "let cond = {|x| $x < 0 }; [-1 -2 9 1] | take while $cond",
                result: Some(Value::test_list(vec![
                    Value::test_int(-1),
                    Value::test_int(-2),
                ])),
            },
            Example {
                description: "Take while the field value is negative",
                example: "[{a: -1} {a: -2} {a: 9} {a: 1}] | take while {|x| $x.a < 0 }",
                result: Some(Value::test_list(vec![
                    Value::test_record(record! {
                        "a" => Value::test_int(-1),
                    }),
                    Value::test_record(record! {
                        "a" => Value::test_int(-2),
                    }),
                ])),
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
        let metadata = input.metadata();
        let span = call.head;

        let capture_block: Closure = call.req(engine_state, stack, 0)?;

        let block = engine_state.get_block(capture_block.block_id).clone();
        let var_id = block.signature.get_positional(0).and_then(|arg| arg.var_id);

        let mut stack = stack.captures_to_stack(capture_block.captures);

        let ctrlc = engine_state.ctrlc.clone();
        let engine_state = engine_state.clone();

        let redirect_stdout = call.redirect_stdout;
        let redirect_stderr = call.redirect_stderr;

        Ok(input
            .into_iter_strict(span)?
            .take_while(move |value| {
                if let Some(var_id) = var_id {
                    stack.add_var(var_id, value.clone());
                }

                eval_block(
                    &engine_state,
                    &mut stack,
                    &block,
                    PipelineData::empty(),
                    redirect_stdout,
                    redirect_stderr,
                )
                .map_or(false, |pipeline_data| {
                    pipeline_data.into_value(span).is_true()
                })
            })
            .into_pipeline_data_with_metadata(metadata, ctrlc))
    }
}

#[cfg(test)]
mod tests {
    use crate::TakeWhile;

    #[test]
    fn test_examples() {
        use crate::test_examples;

        test_examples(TakeWhile)
    }
}
