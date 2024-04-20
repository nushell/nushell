use nu_engine::command_prelude::*;

#[derive(Clone)]
pub struct Skip;

impl Command for Skip {
    fn name(&self) -> &str {
        "skip"
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
            .optional("n", SyntaxShape::Int, "The number of elements to skip.")
            .category(Category::Filters)
    }

    fn usage(&self) -> &str {
        "Skip the first several rows of the input. Counterpart of `drop`. Opposite of `first`."
    }

    fn extra_usage(&self) -> &str {
        r#"To skip specific numbered rows, try `drop nth`. To skip specific named columns, try `reject`."#
    }

    fn search_terms(&self) -> Vec<&str> {
        vec!["ignore", "remove", "last", "slice", "tail"]
    }

    fn examples(&self) -> Vec<Example> {
        vec![
            Example {
                description: "Skip the first value of a list",
                example: "[2 4 6 8] | skip 1",
                result: Some(Value::test_list(vec![
                    Value::test_int(4),
                    Value::test_int(6),
                    Value::test_int(8),
                ])),
            },
            Example {
                description: "Skip two rows of a table",
                example: "[[editions]; [2015] [2018] [2021]] | skip 2",
                result: Some(Value::test_list(vec![Value::test_record(record! {
                    "editions" => Value::test_int(2021),
                })])),
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
        let n: Option<Value> = call.opt(engine_state, stack, 0)?;
        let metadata = input.metadata();

        let n: usize = match n {
            Some(v) => {
                let span = v.span();
                match v {
                    Value::Int { val, .. } => {
                        val.try_into().map_err(|err| ShellError::TypeMismatch {
                            err_message: format!("Could not convert {val} to unsigned int: {err}"),
                            span,
                        })?
                    }
                    _ => {
                        return Err(ShellError::TypeMismatch {
                            err_message: "expected int".into(),
                            span,
                        })
                    }
                }
            }
            None => 1,
        };

        let ctrlc = engine_state.ctrlc.clone();
        let input_span = input.span().unwrap_or(call.head);
        match input {
            PipelineData::ExternalStream { .. } => Err(ShellError::OnlySupportsThisInputType {
                exp_input_type: "list, binary or range".into(),
                wrong_type: "raw data".into(),
                dst_span: call.head,
                src_span: input_span,
            }),
            PipelineData::Value(Value::Binary { val, .. }, metadata) => {
                let bytes = val.into_iter().skip(n).collect::<Vec<_>>();

                Ok(Value::binary(bytes, input_span).into_pipeline_data_with_metadata(metadata))
            }
            _ => Ok(input
                .into_iter_strict(call.head)?
                .skip(n)
                .into_pipeline_data_with_metadata(metadata, ctrlc)),
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::Skip;

    #[test]
    fn test_examples() {
        use crate::test_examples;

        test_examples(Skip {})
    }
}
