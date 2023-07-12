use nu_engine::CallExt;
use nu_protocol::ast::{Call, Expr, Expression};
use nu_protocol::engine::{Command, EngineState, Stack};
use nu_protocol::{
    record, Category, DataSource, Example, IntoPipelineData, PipelineData, PipelineMetadata,
    Record, ShellError, Signature, Span, SyntaxShape, Type, Value,
};

#[derive(Clone)]
pub struct Metadata;

impl Command for Metadata {
    fn name(&self) -> &str {
        "metadata"
    }

    fn usage(&self) -> &str {
        "Get the metadata for items in the stream."
    }

    fn signature(&self) -> nu_protocol::Signature {
        Signature::build("metadata")
            .input_output_types(vec![(Type::Nothing, Type::Record(vec![]))])
            .allow_variants_without_examples(true)
            .optional(
                "expression",
                SyntaxShape::Any,
                "the expression you want metadata for",
            )
            .category(Category::Debug)
    }

    fn run(
        &self,
        engine_state: &EngineState,
        stack: &mut Stack,
        call: &Call,
        input: PipelineData,
    ) -> Result<PipelineData, ShellError> {
        let arg = call.positional_nth(0);
        let head = call.head;

        match arg {
            Some(Expression {
                expr: Expr::FullCellPath(full_cell_path),
                span,
                ..
            }) => {
                if full_cell_path.tail.is_empty() {
                    match &full_cell_path.head {
                        Expression {
                            expr: Expr::Var(var_id),
                            ..
                        } => {
                            let origin = stack.get_var_with_origin(*var_id, *span)?;

                            Ok(build_metadata_record(&origin, &input.metadata(), head)
                                .into_pipeline_data())
                        }
                        _ => {
                            let val: Value = call.req(engine_state, stack, 0)?;
                            Ok(build_metadata_record(&val, &input.metadata(), head)
                                .into_pipeline_data())
                        }
                    }
                } else {
                    let val: Value = call.req(engine_state, stack, 0)?;
                    Ok(build_metadata_record(&val, &input.metadata(), head).into_pipeline_data())
                }
            }
            Some(_) => {
                let val: Value = call.req(engine_state, stack, 0)?;
                Ok(build_metadata_record(&val, &input.metadata(), head).into_pipeline_data())
            }
            None => {
                let mut record = Record::new();
                if let Some(PipelineMetadata { data_source }) = input.metadata().as_deref() {
                    match data_source {
                        DataSource::Ls => record.push("source", Value::string("ls", head)),
                        DataSource::HtmlThemes => {
                            record.push("source", Value::string("into html --list", head))
                        }
                        DataSource::Profiling(values) => {
                            record.push("profiling", Value::list(values.clone(), head))
                        }
                    }
                }

                Ok(Value::record(record, head).into_pipeline_data())
            }
        }
    }

    fn examples(&self) -> Vec<Example> {
        vec![
            Example {
                description: "Get the metadata of a variable",
                example: "let a = 42; metadata $a",
                result: None,
            },
            Example {
                description: "Get the metadata of the input",
                example: "ls | metadata",
                result: None,
            },
        ]
    }
}

fn build_metadata_record(
    arg: &Value,
    metadata: &Option<Box<PipelineMetadata>>,
    head: Span,
) -> Value {
    let mut record = Record::new();

    if let Ok(span) = arg.span() {
        record.push(
            "span",
            Value::record(
                record! {
                    "start" => Value::int(span.start as i64, span),
                    "end" => Value::int(span.end as i64, span),
                },
                span,
            ),
        )
    }

    if let Some(PipelineMetadata { data_source }) = metadata.as_deref() {
        match data_source {
            DataSource::Ls => record.push("source", Value::string("ls", head)),
            DataSource::HtmlThemes => {
                record.push("source", Value::string("into html --list", head))
            }
            DataSource::Profiling(values) => {
                record.push("profiling", Value::list(values.clone(), head))
            }
        }
    }

    Value::record(record, head)
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test_examples() {
        use crate::test_examples;

        test_examples(Metadata {})
    }
}
