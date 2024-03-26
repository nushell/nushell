use nu_engine::command_prelude::*;

#[derive(Clone)]
pub struct SubCommand;

impl Command for SubCommand {
    fn name(&self) -> &str {
        "math sqrt"
    }

    fn signature(&self) -> Signature {
        Signature::build("math sqrt")
            .input_output_types(vec![
                (Type::Number, Type::Float),
                (
                    Type::List(Box::new(Type::Number)),
                    Type::List(Box::new(Type::Float)),
                ),
            ])
            .allow_variants_without_examples(true)
            .category(Category::Math)
    }

    fn usage(&self) -> &str {
        "Returns the square root of the input number."
    }

    fn search_terms(&self) -> Vec<&str> {
        vec!["square", "root"]
    }

    fn run(
        &self,
        engine_state: &EngineState,
        _stack: &mut Stack,
        call: &Call,
        input: PipelineData,
    ) -> Result<PipelineData, ShellError> {
        let head = call.head;
        // This doesn't match explicit nulls
        if matches!(input, PipelineData::Empty) {
            return Err(ShellError::PipelineEmpty { dst_span: head });
        }
        input.map(
            move |value| operate(value, head),
            engine_state.ctrlc.clone(),
        )
    }

    fn examples(&self) -> Vec<Example> {
        vec![Example {
            description: "Compute the square root of each number in a list",
            example: "[9 16] | math sqrt",
            result: Some(Value::list(
                vec![Value::test_float(3.0), Value::test_float(4.0)],
                Span::test_data(),
            )),
        }]
    }
}

fn operate(value: Value, head: Span) -> Value {
    let span = value.span();
    match value {
        Value::Int { val, .. } => {
            let squared = (val as f64).sqrt();
            if squared.is_nan() {
                return error_negative_sqrt(head, span);
            }
            Value::float(squared, span)
        }
        Value::Float { val, .. } => {
            let squared = val.sqrt();
            if squared.is_nan() {
                return error_negative_sqrt(head, span);
            }
            Value::float(squared, span)
        }
        Value::Error { .. } => value,
        other => Value::error(
            ShellError::OnlySupportsThisInputType {
                exp_input_type: "numeric".into(),
                wrong_type: other.get_type().to_string(),
                dst_span: head,
                src_span: other.span(),
            },
            head,
        ),
    }
}

fn error_negative_sqrt(head: Span, span: Span) -> Value {
    Value::error(
        ShellError::UnsupportedInput {
            msg: String::from("Can't square root a negative number"),
            input: "value originates from here".into(),
            msg_span: head,
            input_span: span,
        },
        span,
    )
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test_examples() {
        use crate::test_examples;

        test_examples(SubCommand {})
    }
}
