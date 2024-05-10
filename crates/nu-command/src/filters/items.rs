use super::utils::chain_error_with_input;
use nu_engine::{command_prelude::*, ClosureEval};
use nu_protocol::engine::Closure;

#[derive(Clone)]
pub struct Items;

impl Command for Items {
    fn name(&self) -> &str {
        "items"
    }

    fn signature(&self) -> Signature {
        Signature::build(self.name())
            .input_output_types(vec![(Type::record(), Type::Any)])
            .required(
                "closure",
                SyntaxShape::Closure(Some(vec![SyntaxShape::Any, SyntaxShape::Any])),
                "The closure to run.",
            )
            .allow_variants_without_examples(true)
            .category(Category::Filters)
    }

    fn usage(&self) -> &str {
        "Given a record, iterate on each pair of column name and associated value."
    }

    fn extra_usage(&self) -> &str {
        "This is a the fusion of `columns`, `values` and `each`."
    }

    fn run(
        &self,
        engine_state: &EngineState,
        stack: &mut Stack,
        call: &Call,
        input: PipelineData,
    ) -> Result<PipelineData, ShellError> {
        let head = call.head;
        let closure: Closure = call.req(engine_state, stack, 0)?;

        let metadata = input.metadata();
        match input {
            PipelineData::Empty => Ok(PipelineData::Empty),
            PipelineData::Value(value, ..) => {
                let span = value.span();
                match value {
                    Value::Record { val, .. } => {
                        let mut closure = ClosureEval::new(engine_state, stack, closure);
                        Ok(val
                            .into_owned()
                            .into_iter()
                            .map_while(move |(col, val)| {
                                let result = closure
                                    .add_arg(Value::string(col, span))
                                    .add_arg(val)
                                    .run_with_input(PipelineData::Empty);

                                match result {
                                    Ok(data) => Some(data.into_value(head)),
                                    Err(ShellError::Break { .. }) => None,
                                    Err(err) => {
                                        let err = chain_error_with_input(err, false, span);
                                        Some(Value::error(err, head))
                                    }
                                }
                            })
                            .into_pipeline_data(head, engine_state.ctrlc.clone()))
                    }
                    Value::Error { error, .. } => Err(*error),
                    other => Err(ShellError::OnlySupportsThisInputType {
                        exp_input_type: "record".into(),
                        wrong_type: other.get_type().to_string(),
                        dst_span: head,
                        src_span: other.span(),
                    }),
                }
            }
            PipelineData::ListStream(..) => Err(ShellError::OnlySupportsThisInputType {
                exp_input_type: "record".into(),
                wrong_type: "stream".into(),
                dst_span: head,
                src_span: head,
            }),
            PipelineData::ExternalStream { span, .. } => {
                Err(ShellError::OnlySupportsThisInputType {
                    exp_input_type: "record".into(),
                    wrong_type: "raw data".into(),
                    dst_span: head,
                    src_span: span,
                })
            }
        }
        .map(|data| data.set_metadata(metadata))
    }

    fn examples(&self) -> Vec<Example> {
        vec![Example {
            example:
                "{ new: york, san: francisco } | items {|key, value| echo $'($key) ($value)' }",
            description: "Iterate over each key-value pair of a record",
            result: Some(Value::list(
                vec![
                    Value::test_string("new york"),
                    Value::test_string("san francisco"),
                ],
                Span::test_data(),
            )),
        }]
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test_examples() {
        use crate::test_examples;

        test_examples(Items {})
    }
}
