use nu_engine::command_prelude::*;

#[derive(Clone)]
pub struct Default;

impl Command for Default {
    fn name(&self) -> &str {
        "default"
    }

    fn signature(&self) -> Signature {
        Signature::build("default")
            // TODO: Give more specific type signature?
            // TODO: Declare usage of cell paths in signature? (It seems to behave as if it uses cell paths)
            .input_output_types(vec![(Type::Any, Type::Any)])
            .required(
                "default value",
                SyntaxShape::Any,
                "The value to use as a default.",
            )
            .optional(
                "column name",
                SyntaxShape::String,
                "The name of the column.",
            )
            .category(Category::Filters)
    }

    fn usage(&self) -> &str {
        "Sets a default row's column if missing."
    }

    fn run(
        &self,
        engine_state: &EngineState,
        stack: &mut Stack,
        call: &Call,
        input: PipelineData,
    ) -> Result<PipelineData, ShellError> {
        default(engine_state, stack, call, input)
    }

    fn examples(&self) -> Vec<Example> {
        vec![
            Example {
                description: "Give a default 'target' column to all file entries",
                example: "ls -la | default 'nothing' target ",
                result: None,
            },
            Example {
                description:
                    "Get the env value of `MY_ENV` with a default value 'abc' if not present",
                example: "$env | get --ignore-errors MY_ENV | default 'abc'",
                result: None, // Some(Value::test_string("abc")),
            },
            Example {
                description: "Replace the `null` value in a list",
                example: "[1, 2, null, 4] | default 3",
                result: Some(Value::list(
                    vec![
                        Value::test_int(1),
                        Value::test_int(2),
                        Value::test_int(3),
                        Value::test_int(4),
                    ],
                    Span::test_data(),
                )),
            },
        ]
    }
}

fn default(
    engine_state: &EngineState,
    stack: &mut Stack,
    call: &Call,
    input: PipelineData,
) -> Result<PipelineData, ShellError> {
    let metadata = input.metadata();
    let value: Value = call.req(engine_state, stack, 0)?;
    let column: Option<Spanned<String>> = call.opt(engine_state, stack, 1)?;

    let ctrlc = engine_state.ctrlc.clone();

    if let Some(column) = column {
        input
            .map(
                move |mut item| match item {
                    Value::Record {
                        val: ref mut record,
                        ..
                    } => {
                        let mut found = false;

                        for (col, val) in record.to_mut().iter_mut() {
                            if *col == column.item {
                                found = true;
                                if matches!(val, Value::Nothing { .. }) {
                                    *val = value.clone();
                                }
                            }
                        }

                        if !found {
                            record.to_mut().push(column.item.clone(), value.clone());
                        }

                        item
                    }
                    _ => item,
                },
                ctrlc,
            )
            .map(|x| x.set_metadata(metadata))
    } else if input.is_nothing() {
        Ok(value.into_pipeline_data())
    } else {
        input
            .map(
                move |item| match item {
                    Value::Nothing { .. } => value.clone(),
                    x => x,
                },
                ctrlc,
            )
            .map(|x| x.set_metadata(metadata))
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test_examples() {
        use crate::test_examples;

        test_examples(Default {})
    }
}
