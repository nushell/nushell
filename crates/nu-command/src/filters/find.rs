use nu_engine::{eval_block, CallExt};
use nu_protocol::{
    ast::Call,
    engine::{CaptureBlock, Command, EngineState, Stack},
    Category, Example, IntoInterruptiblePipelineData, PipelineData, ShellError, Signature, Span,
    SyntaxShape, Value,
};

#[derive(Clone)]
pub struct Find;

impl Command for Find {
    fn name(&self) -> &str {
        "find"
    }

    fn signature(&self) -> Signature {
        Signature::build(self.name())
            .named(
                "predicate",
                SyntaxShape::Block(Some(vec![SyntaxShape::Any])),
                "the predicate to satisfy",
                Some('p'),
            )
            .rest("rest", SyntaxShape::Any, "terms to search")
            .category(Category::Filters)
    }

    fn usage(&self) -> &str {
        "Searches terms in the input or for elements of the input that satisfies the predicate."
    }

    fn examples(&self) -> Vec<Example> {
        vec![
            Example {
                description: "Search for multiple terms in a command output",
                example: r#"ls | find toml md sh"#,
                result: None,
            },
            Example {
                description: "Search for a term in a string",
                example: r#"echo Cargo.toml | find toml"#,
                result: Some(Value::test_string("Cargo.toml".to_owned()))
            },
            Example {
                description: "Search a number or a file size in a list of numbers",
                example: r#"[1 5 3kb 4 3Mb] | find 5 3kb"#,
                result: Some(Value::List {
                    vals: vec![Value::test_int(5), Value::test_filesize(3000)],
                    span: Span::test_data()
                }),
            },
            Example {
                description: "Search a char in a list of string",
                example: r#"[moe larry curly] | find l"#,
                result: Some(Value::List {
                    vals: vec![Value::test_string("larry"), Value::test_string("curly")],
                    span: Span::test_data()
                })
            },
            Example {
                description: "Find the first odd value",
                example: "echo [2 4 3 6 5 8] | find --predicate { |it| ($it mod 2) == 1 }",
                result: Some(Value::List {
                    vals: vec![Value::test_int(3), Value::test_int(5)],
                    span: Span::test_data()
                })
            },
            Example {
                description: "Find if a service is not running",
                example: "echo [[version patch]; [0.1.0 $false] [0.1.1 $true] [0.2.0 $false]] | find -p { |it| $it.patch }",
                result: Some(Value::List {
                    vals: vec![Value::test_record(
                            vec!["version", "patch"],
                            vec![Value::test_string("0.1.1"), Value::test_bool(true)]
                        )],
                    span: Span::test_data()
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
        let ctrlc = engine_state.ctrlc.clone();
        let engine_state = engine_state.clone();
        let metadata = input.metadata();
        let config = stack.get_config()?;

        match call.get_flag::<CaptureBlock>(&engine_state, stack, "predicate")? {
            Some(predicate) => {
                let capture_block = predicate;
                let block_id = capture_block.block_id;

                if !call.rest::<Value>(&engine_state, stack, 0)?.is_empty() {
                    return Err(ShellError::IncompatibleParametersSingle(
                        "expected either a predicate or terms, not both".to_owned(),
                        span,
                    ));
                }

                let block = engine_state.get_block(block_id).clone();
                let var_id = block.signature.get_positional(0).and_then(|arg| arg.var_id);

                let mut stack = stack.captures_to_stack(&capture_block.captures);

                input.filter(
                    move |value| {
                        if let Some(var_id) = var_id {
                            stack.add_var(var_id, value.clone());
                        }

                        eval_block(
                            &engine_state,
                            &mut stack,
                            &block,
                            PipelineData::new_with_metadata(metadata.clone(), span),
                        )
                        .map_or(false, |pipeline_data| {
                            pipeline_data.into_value(span).is_true()
                        })
                    },
                    ctrlc,
                )
            }
            None => {
                let terms = call.rest::<Value>(&engine_state, stack, 0)?;
                let lower_terms = terms
                    .iter()
                    .map(|v| {
                        if let Ok(span) = v.span() {
                            Value::string(v.into_string("", &config).to_lowercase(), span)
                        } else {
                            v.clone()
                        }
                    })
                    .collect::<Vec<Value>>();

                let pipe = input.filter(
                    move |value| {
                        let lower_value = if let Ok(span) = value.span() {
                            Value::string(value.into_string("", &config).to_lowercase(), span)
                        } else {
                            value.clone()
                        };
                        lower_terms.iter().any(|term| match value {
                            Value::Bool { .. }
                            | Value::Int { .. }
                            | Value::Filesize { .. }
                            | Value::Duration { .. }
                            | Value::Date { .. }
                            | Value::Range { .. }
                            | Value::Float { .. }
                            | Value::Block { .. }
                            | Value::Nothing { .. }
                            | Value::Error { .. } => lower_value
                                .eq(span, term)
                                .map_or(false, |value| value.is_true()),
                            Value::String { .. }
                            | Value::List { .. }
                            | Value::CellPath { .. }
                            | Value::CustomValue { .. } => term
                                .r#in(span, &lower_value)
                                .map_or(false, |value| value.is_true()),
                            Value::Record { vals, .. } => vals.iter().any(|val| {
                                if let Ok(span) = val.span() {
                                    let lower_val = Value::string(
                                        val.into_string("", &config).to_lowercase(),
                                        Span::test_data(),
                                    );

                                    term.r#in(span, &lower_val)
                                        .map_or(false, |value| value.is_true())
                                } else {
                                    term.r#in(span, val).map_or(false, |value| value.is_true())
                                }
                            }),
                            Value::Binary { .. } => false,
                        })
                    },
                    ctrlc,
                )?;
                match metadata {
                    Some(m) => {
                        Ok(pipe.into_pipeline_data_with_metadata(m, engine_state.ctrlc.clone()))
                    }
                    None => Ok(pipe),
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_examples() {
        use crate::test_examples;

        test_examples(Find)
    }
}
