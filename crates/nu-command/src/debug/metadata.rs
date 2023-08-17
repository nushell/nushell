use nu_engine::CallExt;
use nu_protocol::ast::{Call, Expr, Expression};
use nu_protocol::engine::{Command, EngineState, Stack};
use nu_protocol::{
    Category, DataSource, Example, IntoPipelineData, PipelineData, PipelineMetadata, ShellError,
    Signature, Span, SpannedValue, SyntaxShape, Type,
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
            .input_output_types(vec![(Type::Any, Type::Record(vec![]))])
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
                            let val: SpannedValue = call.req(engine_state, stack, 0)?;
                            Ok(build_metadata_record(&val, &input.metadata(), head)
                                .into_pipeline_data())
                        }
                    }
                } else {
                    let val: SpannedValue = call.req(engine_state, stack, 0)?;
                    Ok(build_metadata_record(&val, &input.metadata(), head).into_pipeline_data())
                }
            }
            Some(_) => {
                let val: SpannedValue = call.req(engine_state, stack, 0)?;
                Ok(build_metadata_record(&val, &input.metadata(), head).into_pipeline_data())
            }
            None => {
                let mut cols = vec![];
                let mut vals = vec![];
                if let Some(x) = input.metadata().as_deref() {
                    match x {
                        PipelineMetadata {
                            data_source: DataSource::Ls,
                        } => {
                            cols.push("source".into());
                            vals.push(SpannedValue::string("ls", head))
                        }
                        PipelineMetadata {
                            data_source: DataSource::HtmlThemes,
                        } => {
                            cols.push("source".into());
                            vals.push(SpannedValue::string("into html --list", head))
                        }
                        PipelineMetadata {
                            data_source: DataSource::Profiling(values),
                        } => {
                            cols.push("profiling".into());
                            vals.push(SpannedValue::list(values.clone(), head))
                        }
                    }
                }

                Ok(SpannedValue::Record {
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
    arg: &SpannedValue,
    metadata: &Option<Box<PipelineMetadata>>,
    head: Span,
) -> SpannedValue {
    let mut cols = vec![];
    let mut vals = vec![];

    let span = arg.span();
    cols.push("span".into());
    vals.push(SpannedValue::Record {
        cols: vec!["start".into(), "end".into()],
        vals: vec![
            SpannedValue::Int {
                val: span.start as i64,
                span,
            },
            SpannedValue::Int {
                val: span.end as i64,
                span,
            },
        ],
        span: head,
    });

    if let Some(x) = metadata.as_deref() {
        match x {
            PipelineMetadata {
                data_source: DataSource::Ls,
            } => {
                cols.push("source".into());
                vals.push(SpannedValue::string("ls", head))
            }
            PipelineMetadata {
                data_source: DataSource::HtmlThemes,
            } => {
                cols.push("source".into());
                vals.push(SpannedValue::string("into html --list", head))
            }
            PipelineMetadata {
                data_source: DataSource::Profiling(values),
            } => {
                cols.push("profiling".into());
                vals.push(SpannedValue::list(values.clone(), head))
            }
        }
    }

    SpannedValue::Record {
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
