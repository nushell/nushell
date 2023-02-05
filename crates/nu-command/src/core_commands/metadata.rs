use nu_engine::CallExt;
use nu_protocol::ast::{Call, Expr, Expression};
use nu_protocol::engine::{Command, EngineState, Stack};
use nu_protocol::{
    Category, Example, IntoPipelineData, PipelineData, Signature, Span, SyntaxShape, Type, Value,
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
            .switch(
                "data",
                "also add the data to the output record, under the key `data`",
                Some('d'),
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
        let include_data = call.has_flag("data");

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
                            Ok(build_metadata_record(
                                origin.span().ok(),
                                input,
                                include_data,
                                head,
                            ))
                        }
                        _ => {
                            let val: Value = call.req(engine_state, stack, 0)?;
                            Ok(build_metadata_record(
                                val.span().ok(),
                                input,
                                include_data,
                                head,
                            ))
                        }
                    }
                } else {
                    let val: Value = call.req(engine_state, stack, 0)?;
                    Ok(build_metadata_record(
                        val.span().ok(),
                        input,
                        include_data,
                        head,
                    ))
                }
            }
            Some(_) => {
                let val: Value = call.req(engine_state, stack, 0)?;
                Ok(build_metadata_record(
                    val.span().ok(),
                    input,
                    include_data,
                    head,
                ))
            }
            None => Ok(build_metadata_record(None, input, include_data, head)),
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
            Example {
                description: "Get the metadata of the input, along with the data",
                example: "ls | metadata --data",
                result: None,
            },
        ]
    }
}

pub fn build_metadata_record(
    arg_span: Option<Span>,
    pipeline: PipelineData,
    include_data: bool,
    head: Span,
) -> PipelineData {
    let mut cols = vec![];
    let mut vals = vec![];
    let metadata = pipeline.metadata();

    if include_data {
        cols.push("data".into());
        vals.push(pipeline.into_value(head));
    }

    if let Some(span) = arg_span {
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
        cols.push("source".into());
        vals.push(Value::string(format!("{}", x), head))
    }

    Value::Record {
        cols,
        vals,
        span: head,
    }
    .into_pipeline_data()
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
