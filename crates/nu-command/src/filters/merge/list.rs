use super::common::{do_merge, MergeStrategy};
use nu_engine::command_prelude::*;

#[derive(Clone)]
pub struct MergeList;

impl Command for MergeList {
    fn name(&self) -> &str {
        "merge list"
    }

    fn description(&self) -> &str {
        "TODO(rose)"
    }

    fn extra_description(&self) -> &str {
        r#"TODO(rose)"#
    }

    fn signature(&self) -> nu_protocol::Signature {
        Signature::build("merge list")
            .input_output_types(vec![
                (Type::record(), Type::record()),
                (Type::table(), Type::table()),
                (Type::list(Type::Any), Type::list(Type::Any)),
            ])
            .required(
                "value",
                SyntaxShape::OneOf(vec![
                    SyntaxShape::Record(vec![]),
                    SyntaxShape::Table(vec![]),
                    SyntaxShape::List(SyntaxShape::Any.into()),
                ]),
                "The new value to merge with.",
            )
            .category(Category::Filters)
    }

    fn examples(&self) -> Vec<Example> {
        // TODO(rose)
        vec![]
    }

    fn run(
        &self,
        engine_state: &EngineState,
        stack: &mut Stack,
        call: &Call,
        input: PipelineData,
    ) -> Result<PipelineData, ShellError> {
        let head = call.head;
        let merge_value: Value = call.req(engine_state, stack, 0)?;
        let metadata = input.metadata();

        // collect input before typechecking, so tables are detected as such
        let input_span = input.span().unwrap_or(head);
        let input = input.into_value(input_span)?;

        match (input.get_type(), merge_value.get_type()) {
            (Type::Record { .. }, Type::Record { .. }) => (),
            (Type::Table { .. }, Type::Table { .. }) => (),
            (Type::List { .. }, Type::List { .. }) => (),
            _ => {
                return Err(ShellError::PipelineMismatch {
                    exp_input_type:
                        "input and argument to be both record, both table, or both list".to_string(),
                    dst_span: head,
                    src_span: input.span(),
                });
            }
        };

        let merged = do_merge(input, merge_value, MergeStrategy::Concatenation, head)?;
        Ok(merged.into_pipeline_data_with_metadata(metadata))
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test_examples() {
        use crate::test_examples;

        test_examples(MergeList {})
    }
}
