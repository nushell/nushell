use nu_engine::command_prelude::*;
use nu_parser::parse;
use nu_protocol::engine::StateWorkingSet;

#[derive(Clone)]
pub struct Ast;

impl Command for Ast {
    fn name(&self) -> &str {
        "ast"
    }

    fn usage(&self) -> &str {
        "Print the abstract syntax tree (ast) for a pipeline."
    }

    fn signature(&self) -> Signature {
        Signature::build("ast")
            .input_output_types(vec![(Type::String, Type::record())])
            .required(
                "pipeline",
                SyntaxShape::String,
                "The pipeline to print the ast for.",
            )
            .switch("json", "serialize to json", Some('j'))
            .switch("minify", "minify the nuon or json output", Some('m'))
            .allow_variants_without_examples(true)
            .category(Category::Debug)
    }

    fn run(
        &self,
        engine_state: &EngineState,
        stack: &mut Stack,
        call: &Call,
        _input: PipelineData,
    ) -> Result<PipelineData, ShellError> {
        let pipeline: Spanned<String> = call.req(engine_state, stack, 0)?;
        let to_json = call.has_flag(engine_state, stack, "json")?;
        let minify = call.has_flag(engine_state, stack, "minify")?;
        let mut working_set = StateWorkingSet::new(engine_state);
        let block_output = parse(&mut working_set, None, pipeline.item.as_bytes(), false);
        let error_output = working_set.parse_errors.first();
        let block_span = match &block_output.span {
            Some(span) => span,
            None => &pipeline.span,
        };
        if to_json {
            // Get the block as json
            let serde_block_str = if minify {
                serde_json::to_string(&*block_output)
            } else {
                serde_json::to_string_pretty(&*block_output)
            };
            let block_json = match serde_block_str {
                Ok(json) => json,
                Err(e) => Err(ShellError::CantConvert {
                    to_type: "string".to_string(),
                    from_type: "block".to_string(),
                    span: *block_span,
                    help: Some(format!(
                        "Error: {e}\nCan't convert {block_output:?} to string"
                    )),
                })?,
            };
            // Get the error as json
            let serde_error_str = if minify {
                serde_json::to_string(&error_output)
            } else {
                serde_json::to_string_pretty(&error_output)
            };

            let error_json = match serde_error_str {
                Ok(json) => json,
                Err(e) => Err(ShellError::CantConvert {
                    to_type: "string".to_string(),
                    from_type: "error".to_string(),
                    span: *block_span,
                    help: Some(format!(
                        "Error: {e}\nCan't convert {error_output:?} to string"
                    )),
                })?,
            };

            // Create a new output record, merging the block and error
            let output_record = Value::record(
                record! {
                    "block" => Value::string(block_json, *block_span),
                    "error" => Value::string(error_json, Span::test_data()),
                },
                pipeline.span,
            );
            Ok(output_record.into_pipeline_data())
        } else {
            let block_value = Value::string(
                if minify {
                    format!("{block_output:?}")
                } else {
                    format!("{block_output:#?}")
                },
                pipeline.span,
            );
            let error_value = Value::string(
                if minify {
                    format!("{error_output:?}")
                } else {
                    format!("{error_output:#?}")
                },
                pipeline.span,
            );
            let output_record = Value::record(
                record! {
                    "block" => block_value,
                    "error" => error_value
                },
                pipeline.span,
            );
            Ok(output_record.into_pipeline_data())
        }
    }

    fn examples(&self) -> Vec<Example> {
        vec![
            Example {
                description: "Print the ast of a string",
                example: "ast 'hello'",
                result: None,
            },
            Example {
                description: "Print the ast of a pipeline",
                example: "ast 'ls | where name =~ README'",
                result: None,
            },
            Example {
                description: "Print the ast of a pipeline with an error",
                example: "ast 'for x in 1..10 { echo $x '",
                result: None,
            },
            Example {
                description:
                    "Print the ast of a pipeline with an error, as json, in a nushell table",
                example: "ast 'for x in 1..10 { echo $x ' --json | get block | from json",
                result: None,
            },
            Example {
                description: "Print the ast of a pipeline with an error, as json, minified",
                example: "ast 'for x in 1..10 { echo $x ' --json --minify",
                result: None,
            },
        ]
    }
}

#[cfg(test)]
mod test {
    #[test]
    fn test_examples() {
        use super::Ast;
        use crate::test_examples;
        test_examples(Ast {})
    }
}
