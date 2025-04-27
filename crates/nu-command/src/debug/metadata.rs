use nu_engine::command_prelude::*;
use nu_protocol::{
    ast::{Expr, Expression},
    DataSource, PipelineMetadata,
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
                                Some(&origin),
                                input.metadata().as_ref(),
                                head,
                            )
                            .into_pipeline_data())
                        }
                        _ => {
                            let val: Value = call.req(engine_state, stack, 0)?;
                            Ok(
                                build_metadata_record(Some(&val), input.metadata().as_ref(), head)
                                    .into_pipeline_data(),
                            )
                        }
                    }
                } else {
                    let val: Value = call.req(engine_state, stack, 0)?;
                    Ok(
                        build_metadata_record(Some(&val), input.metadata().as_ref(), head)
                            .into_pipeline_data(),
                    )
                }
            }
            Some(_) => {
                let val: Value = call.req(engine_state, stack, 0)?;
                Ok(
                    build_metadata_record(Some(&val), input.metadata().as_ref(), head)
                        .into_pipeline_data(),
                )
            }
            None => {
                Ok(build_metadata_record(None, input.metadata().as_ref(), head)
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

pub(super) fn build_metadata_record(
    arg: Option<&Value>,
    metadata: Option<&PipelineMetadata>,
    head: Span,
) -> Value {
    let mut record = Record::new();

    if let Some(arg) = arg {
        let span = arg.span();
        record.push(
            "span",
            Value::record(
                record! {
                    "start" => Value::int(span.start as i64,span),
                    "end" => Value::int(span.end as i64, span),
                },
                head,
            ),
        );
    }

    if let Some(x) = metadata {
        let source: Option<String> = match &x.data_source {
            DataSource::Ls => Some("ls".into()),
            DataSource::HtmlThemes => Some("to html --list".into()),
            DataSource::FilePath(path) => Some(path.to_string_lossy().into()),
            DataSource::Uri(uri) => Some(uri.into()),
            DataSource::None => None,
        };

        if let Some(source) = source {
            record.push("source", Value::string(source, head));
        }

        if let Some(ref content_type) = x.content_type {
            record.push("content_type", Value::string(content_type, head));
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
