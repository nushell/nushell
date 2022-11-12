use nu_engine::CallExt;
use nu_protocol::ast::{Call, Expr, Expression};
use nu_protocol::engine::{Command, EngineState, Stack};
use nu_protocol::{
    Category, DataSource, Example, IntoPipelineData, PipelineData, PipelineMetadata, Signature,
    Span, SyntaxShape, Type, Value,
};

#[derive(Clone)]
pub struct Metadata;

impl Command for Metadata {
    fn name(&self) -> &str {
        "metadata"
    }

    fn usage(&self) -> &str {
        "Get the metadata for items in the stream"
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
            .category(Category::Core)
    }

    fn run(
        &self,
        engine_state: &EngineState,
        stack: &mut Stack,
        call: &Call,
        input: PipelineData,
    ) -> Result<nu_protocol::PipelineData, nu_protocol::ShellError> {
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
                let mut cols = vec![];
                let mut vals = vec![];
                if let Some(x) = &input.metadata() {
                    match x {
                        PipelineMetadata {
                            data_source: DataSource::Ls,
                        } => {
                            cols.push("source".into());
                            vals.push(Value::String {
                                val: "ls".into(),
                                span: head,
                            })
                        }
                    }
                }

                Ok(Value::Record {
                    cols,
                    vals,
                    span: head,
                }
                .into_pipeline_data())
            }
        }
    }

    fn examples(&self) -> Vec<Example> {
        vec![
            Example {
                description: "Get the metadata of a variable",
                example: "metadata $a",
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

fn build_metadata_record(arg: &Value, metadata: &Option<PipelineMetadata>, head: Span) -> Value {
    let mut cols = vec![];
    let mut vals = vec![];

    if let Ok(span) = arg.span() {
        cols.push("span".into());
        vals.push(Value::Record {
            cols: vec!["start".into(), "end".into()],
            vals: vec![
                Value::Int {
                    val: span.start as i64,
                    span,
                },
                Value::Int {
                    val: span.end as i64,
                    span,
                },
            ],
            span: head,
        });
    }

    if let Some(x) = &metadata {
        match x {
            PipelineMetadata {
                data_source: DataSource::Ls,
            } => {
                cols.push("source".into());
                vals.push(Value::String {
                    val: "ls".into(),
                    span: head,
                })
            }
        }
    }

    Value::Record {
        cols,
        vals,
        span: head,
    }
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
