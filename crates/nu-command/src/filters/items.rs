use super::utils::chain_error_with_input;
use nu_engine::{eval_block_with_early_return, CallExt};
use nu_protocol::ast::Call;
use nu_protocol::engine::{Closure, Command, EngineState, Stack};
use nu_protocol::{
    Category, Example, IntoInterruptiblePipelineData, PipelineData, ShellError, Signature, Span,
    SyntaxShape, Type, Value,
};

#[derive(Clone)]
pub struct Items;

impl Command for Items {
    fn name(&self) -> &str {
        "items"
    }

    fn signature(&self) -> Signature {
        Signature::build(self.name())
            .input_output_types(vec![(
                Type::Record(vec![]),
                Type::List(Box::new(Type::String)),
            )])
            .required(
                "closure",
                SyntaxShape::Closure(Some(vec![SyntaxShape::Any, SyntaxShape::Any])),
                "the closure to run",
            )
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
        let capture_block: Closure = call.req(engine_state, stack, 0)?;

        let metadata = input.metadata();
        let ctrlc = engine_state.ctrlc.clone();
        let engine_state = engine_state.clone();
        let block = engine_state.get_block(capture_block.block_id).clone();
        let mut stack = stack.captures_to_stack(&capture_block.captures);
        let orig_env_vars = stack.env_vars.clone();
        let orig_env_hidden = stack.env_hidden.clone();
        let span = call.head;
        let redirect_stdout = call.redirect_stdout;
        let redirect_stderr = call.redirect_stderr;

        let input_span = input.span().unwrap_or(call.head);
        let run_for_each_item = move |keyval: (String, Value)| -> Option<Value> {
            // with_env() is used here to ensure that each iteration uses
            // a different set of environment variables.
            // Hence, a 'cd' in the first loop won't affect the next loop.
            stack.with_env(&orig_env_vars, &orig_env_hidden);

            if let Some(var) = block.signature.get_positional(0) {
                if let Some(var_id) = &var.var_id {
                    stack.add_var(*var_id, Value::string(keyval.0.clone(), span));
                }
            }

            if let Some(var) = block.signature.get_positional(1) {
                if let Some(var_id) = &var.var_id {
                    stack.add_var(*var_id, keyval.1);
                }
            }

            match eval_block_with_early_return(
                &engine_state,
                &mut stack,
                &block,
                PipelineData::empty(),
                redirect_stdout,
                redirect_stderr,
            ) {
                Ok(v) => Some(v.into_value(span)),
                Err(ShellError::Break(_)) => None,
                Err(error) => {
                    let error = chain_error_with_input(error, Ok(input_span));
                    Some(Value::Error {
                        error: Box::new(error),
                    })
                }
            }
        };
        match input {
            PipelineData::Empty => Ok(PipelineData::Empty),
            PipelineData::Value(Value::Record { cols, vals, .. }, ..) => Ok(cols
                .into_iter()
                .zip(vals.into_iter())
                .map_while(run_for_each_item)
                .into_pipeline_data(ctrlc)),
            // Errors
            PipelineData::ListStream(..) => Err(ShellError::OnlySupportsThisInputType {
                exp_input_type: "record".into(),
                wrong_type: "stream".into(),
                dst_span: call.head,
                src_span: input_span,
            }),
            PipelineData::Value(Value::Error { error }, ..) => Err(*error),
            PipelineData::Value(other, ..) => Err(ShellError::OnlySupportsThisInputType {
                exp_input_type: "record".into(),
                wrong_type: other.get_type().to_string(),
                dst_span: call.head,
                src_span: other.expect_span(),
            }),
            PipelineData::ExternalStream { .. } => Err(ShellError::OnlySupportsThisInputType {
                exp_input_type: "record".into(),
                wrong_type: "raw data".into(),
                dst_span: call.head,
                src_span: input_span,
            }),
        }
        .map(|x| x.set_metadata(metadata))
    }

    fn examples(&self) -> Vec<Example> {
        vec![Example {
            example:
                "{ new: york, san: francisco } | items {|key, value| echo $'($key) ($value)' }",
            description: "Iterate over each key-value pair of a record",
            result: Some(Value::List {
                vals: vec![
                    Value::test_string("new york"),
                    Value::test_string("san francisco"),
                ],
                span: Span::test_data(),
            }),
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
