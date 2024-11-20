use nu_engine::command_prelude::*;
use nu_parser::{flatten_block, parse};
use nu_protocol::{engine::StateWorkingSet, record};
use serde_json::{json, Value as JsonValue};

#[derive(Clone)]
pub struct Ast;

impl Command for Ast {
    fn name(&self) -> &str {
        "ast"
    }

    fn description(&self) -> &str {
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
            .switch("json", "Serialize to json", Some('j'))
            .switch("minify", "Minify the nuon or json output", Some('m'))
            .switch("flatten", "An easier to read version of the ast", Some('f'))
            .allow_variants_without_examples(true)
            .category(Category::Debug)
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
            Example {
                description: "Print the ast of a string flattened, as json, minified",
                example: "ast 'hello' --flatten --json --minify",
                result: Some(Value::test_string(
                    r#"[{"content":"hello","shape":"shape_external","span":{"start":0,"end":5}}]"#,
                )),
            },
            Example {
                description: "Print the ast of a pipeline flattened, as json, minified",
                example: "ast 'ls | sort-by type name -i' --flatten --json --minify",
                result: Some(Value::test_string(
                    r#"[{"content":"ls","shape":"shape_internalcall","span":{"start":0,"end":2}},{"content":"|","shape":"shape_pipe","span":{"start":3,"end":4}},{"content":"sort-by","shape":"shape_internalcall","span":{"start":5,"end":12}},{"content":"type","shape":"shape_string","span":{"start":13,"end":17}},{"content":"name","shape":"shape_string","span":{"start":18,"end":22}},{"content":"-i","shape":"shape_flag","span":{"start":23,"end":25}}]"#,
                )),
            },
        ]
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
        let flatten = call.has_flag(engine_state, stack, "flatten")?;

        let mut working_set = StateWorkingSet::new(engine_state);
        let offset = working_set.next_span_start();
        let parsed_block = parse(&mut working_set, None, pipeline.item.as_bytes(), false);

        if flatten {
            let flat = flatten_block(&working_set, &parsed_block);
            if to_json {
                let mut json_val: JsonValue = json!([]);
                for (span, shape) in flat {
                    let content =
                        String::from_utf8_lossy(working_set.get_span_contents(span)).to_string();

                    let json = json!(
                        {
                            "content": content,
                            "shape": shape.to_string(),
                            "span": {
                                "start": span.start.checked_sub(offset),
                                "end": span.end.checked_sub(offset),
                            },
                        }
                    );
                    json_merge(&mut json_val, &json);
                }
                let json_string = if minify {
                    if let Ok(json_str) = serde_json::to_string(&json_val) {
                        json_str
                    } else {
                        "{}".to_string()
                    }
                } else if let Ok(json_str) = serde_json::to_string_pretty(&json_val) {
                    json_str
                } else {
                    "{}".to_string()
                };

                Ok(Value::string(json_string, pipeline.span).into_pipeline_data())
            } else {
                // let mut rec: Record = Record::new();
                let mut rec = vec![];
                for (span, shape) in flat {
                    let content =
                        String::from_utf8_lossy(working_set.get_span_contents(span)).to_string();
                    let each_rec = record! {
                        "content" => Value::test_string(content),
                        "shape" => Value::test_string(shape.to_string()),
                        "span" => Value::test_record(record!{
                            "start" => Value::test_int(match span.start.checked_sub(offset) {
                                Some(start) => start as i64,
                                None => 0
                            }),
                            "end" => Value::test_int(match span.end.checked_sub(offset) {
                                Some(end) => end as i64,
                                None => 0
                            }),
                        }),
                    };
                    rec.push(Value::test_record(each_rec));
                }
                Ok(Value::list(rec, pipeline.span).into_pipeline_data())
            }
        } else {
            let error_output = working_set.parse_errors.first();
            let block_span = match &parsed_block.span {
                Some(span) => span,
                None => &pipeline.span,
            };
            if to_json {
                // Get the block as json
                let serde_block_str = if minify {
                    serde_json::to_string(&*parsed_block)
                } else {
                    serde_json::to_string_pretty(&*parsed_block)
                };
                let block_json = match serde_block_str {
                    Ok(json) => json,
                    Err(e) => Err(ShellError::CantConvert {
                        to_type: "string".to_string(),
                        from_type: "block".to_string(),
                        span: *block_span,
                        help: Some(format!(
                            "Error: {e}\nCan't convert {parsed_block:?} to string"
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
                        format!("{parsed_block:?}")
                    } else {
                        format!("{parsed_block:#?}")
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
    }
}

fn json_merge(a: &mut JsonValue, b: &JsonValue) {
    match (a, b) {
        (JsonValue::Object(ref mut a), JsonValue::Object(b)) => {
            for (k, v) in b {
                json_merge(a.entry(k).or_insert(JsonValue::Null), v);
            }
        }
        (JsonValue::Array(ref mut a), JsonValue::Array(b)) => {
            a.extend(b.clone());
        }
        (JsonValue::Array(ref mut a), JsonValue::Object(b)) => {
            a.extend([JsonValue::Object(b.clone())]);
        }
        (a, b) => {
            *a = b.clone();
        }
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
