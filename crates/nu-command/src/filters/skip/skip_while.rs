use nu_engine::{get_eval_block, CallExt};

use nu_protocol::{
    ast::Call,
    engine::{Closure, Command, EngineState, Stack},
    record, Category, Example, IntoInterruptiblePipelineData, PipelineData, ShellError, Signature,
    SyntaxShape, Type, Value,
};

#[derive(Clone)]
pub struct SkipWhile;

impl Command for SkipWhile {
    fn name(&self) -> &str {
        "skip while"
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
                "The predicate that skipped element must match.",
            )
            .category(Category::Filters)
    }

    fn usage(&self) -> &str {
        "Skip elements of the input while a predicate is true."
    }

    fn search_terms(&self) -> Vec<&str> {
        vec!["ignore"]
    }

    fn examples(&self) -> Vec<Example> {
        vec![
            Example {
                description: "Skip while the element is negative",
                example: "[-2 0 2 -1] | skip while {|x| $x < 0 }",
                result: Some(Value::test_list(vec![
                    Value::test_int(0),
                    Value::test_int(2),
                    Value::test_int(-1),
                ])),
            },
            Example {
                description: "Skip while the element is negative using stored condition",
                example: "let cond = {|x| $x < 0 }; [-2 0 2 -1] | skip while $cond",
                result: Some(Value::test_list(vec![
                    Value::test_int(0),
                    Value::test_int(2),
                    Value::test_int(-1),
                ])),
            },
            Example {
                description: "Skip while the field value is negative",
                example: "[{a: -2} {a: 0} {a: 2} {a: -1}] | skip while {|x| $x.a < 0 }",
                result: Some(Value::test_list(vec![
                    Value::test_record(record! {
                        "a" => Value::test_int(0),
                    }),
                    Value::test_record(record! {
                        "a" => Value::test_int(2),
                    }),
                    Value::test_record(record! {
                        "a" => Value::test_int(-1),
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
        let span = call.head;
        let metadata = input.metadata();

        let capture_block: Closure = call.req(engine_state, stack, 0)?;

        let block = engine_state.get_block(capture_block.block_id).clone();
        let var_id = block.signature.get_positional(0).and_then(|arg| arg.var_id);
        let mut stack = stack.captures_to_stack(capture_block.captures);

        let ctrlc = engine_state.ctrlc.clone();
        let engine_state = engine_state.clone();

        let redirect_stdout = call.redirect_stdout;
        let redirect_stderr = call.redirect_stderr;

        let eval_block = get_eval_block(&engine_state);

        Ok(input
            .into_iter_strict(span)?
            .skip_while(move |value| {
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
    use crate::SkipWhile;

    #[test]
    fn test_examples() {
        use crate::test_examples;

        test_examples(SkipWhile)
    }
}
