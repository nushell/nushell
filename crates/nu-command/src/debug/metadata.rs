use super::util::{build_metadata_record, extend_record_with_metadata};
use nu_engine::command_prelude::*;
use nu_protocol::{
    PipelineMetadata,
    ast::{Expr, Expression},
};

#[derive(Clone)]
pub struct Metadata;

impl Command for Metadata {
    fn name(&self) -> &str {
        "metadata"
    }

    fn description(&self) -> &str {
        "Get the metadata for items in the stream."
    }

    fn signature(&self) -> nu_protocol::Signature {
        Signature::build("metadata")
            .input_output_types(vec![(Type::Any, Type::record())])
            .allow_variants_without_examples(true)
            .optional(
                "expression",
                SyntaxShape::Any,
                "The expression you want metadata for.",
            )
            .category(Category::Debug)
    }

    fn requires_ast_for_arguments(&self) -> bool {
        true
    }

    fn run(
        &self,
        engine_state: &EngineState,
        stack: &mut Stack,
        call: &Call,
        input: PipelineData,
    ) -> Result<PipelineData, ShellError> {
        let arg = call.positional_nth(stack, 0);
        let head = call.head;

        if !matches!(input, PipelineData::Empty)
            && let Some(arg_expr) = arg
        {
            return Err(ShellError::IncompatibleParameters {
                left_message: "pipeline input was provided".into(),
                left_span: head,
                right_message: "but a positional metadata expression was also given".into(),
                right_span: arg_expr.span,
            });
        }

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
                            Ok(build_metadata_record_value(
                                &origin,
                                input.metadata().as_ref(),
                                head,
                            )
                            .into_pipeline_data())
                        }
                        _ => {
                            let val: Value = call.req(engine_state, stack, 0)?;
                            Ok(
                                build_metadata_record_value(&val, input.metadata().as_ref(), head)
                                    .into_pipeline_data(),
                            )
                        }
                    }
                } else {
                    let val: Value = call.req(engine_state, stack, 0)?;
                    Ok(
                        build_metadata_record_value(&val, input.metadata().as_ref(), head)
                            .into_pipeline_data(),
                    )
                }
            }
            Some(_) => {
                let val: Value = call.req(engine_state, stack, 0)?;
                Ok(
                    build_metadata_record_value(&val, input.metadata().as_ref(), head)
                        .into_pipeline_data(),
                )
            }
            None => {
                Ok(Value::record(build_metadata_record(&input, head), head).into_pipeline_data())
            }
        }
    }

    fn examples(&self) -> Vec<Example<'_>> {
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

fn build_metadata_record_value(
    arg: &Value,
    metadata: Option<&PipelineMetadata>,
    head: Span,
) -> Value {
    let mut record = Record::new();
    record.push("span", arg.span().into_value(head));
    Value::record(extend_record_with_metadata(record, metadata, head), head)
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
