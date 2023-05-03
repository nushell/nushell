use nu_engine::CallExt;
use nu_protocol::ast::Call;
use nu_protocol::engine::{Command, EngineState, Stack};
use nu_protocol::{
    Category, Example, PipelineData, ShellError, Signature, Span, SyntaxShape, Type, Value,
};

#[derive(Clone)]
pub struct SubCommand;

impl Command for SubCommand {
    fn name(&self) -> &str {
        "math round"
    }

    fn signature(&self) -> Signature {
        Signature::build("math round")
            .input_output_types(vec![(Type::Number, Type::Number)])
            .vectorizes_over_list(true)
            .named(
                "precision",
                SyntaxShape::Number,
                "digits of precision",
                Some('p'),
            )
            .category(Category::Math)
    }

    fn usage(&self) -> &str {
        "Returns the input number rounded to the specified precision."
    }

    fn search_terms(&self) -> Vec<&str> {
        vec!["approx", "closest", "nearest"]
    }

    fn run(
        &self,
        engine_state: &EngineState,
        stack: &mut Stack,
        call: &Call,
        input: PipelineData,
    ) -> Result<PipelineData, ShellError> {
        let precision_param: Option<i64> = call.get_flag(engine_state, stack, "precision")?;
        let head = call.head;
        // This doesn't match explicit nulls
        if matches!(input, PipelineData::Empty) {
            return Err(ShellError::PipelineEmpty { dst_span: head });
        }
        input.map(
            move |value| operate(value, head, precision_param),
            engine_state.ctrlc.clone(),
        )
    }

    fn examples(&self) -> Vec<Example> {
        vec![
            Example {
                description: "Apply the round function to a list of numbers",
                example: "[1.5 2.3 -3.1] | math round",
                result: Some(Value::List {
                    vals: vec![Value::test_int(2), Value::test_int(2), Value::test_int(-3)],
                    span: Span::test_data(),
                }),
            },
            Example {
                description: "Apply the round function with precision specified",
                example: "[1.555 2.333 -3.111] | math round -p 2",
                result: Some(Value::List {
                    vals: vec![
                        Value::test_float(1.56),
                        Value::test_float(2.33),
                        Value::test_float(-3.11),
                    ],
                    span: Span::test_data(),
                }),
            },
            Example {
                description: "Apply negative precision to a list of numbers",
                example: "[123, 123.3, -123.4] | math round -p -1",
                result: Some(Value::List {
                    vals: vec![
                        Value::test_int(120),
                        Value::test_int(120),
                        Value::test_int(-120),
                    ],
                    span: Span::test_data(),
                }),
            },
        ]
    }
}

fn operate(value: Value, head: Span, precision: Option<i64>) -> Value {
    // We treat int values as float values in order to avoid code repetition in the match closure
    let value = if let Value::Int { val, span } = value {
        Value::Float {
            val: val as f64,
            span,
        }
    } else {
        value
    };

    match value {
        Value::Float { val, span } => match precision {
            Some(precision_number) => Value::Float {
                val: ((val * ((10_f64).powf(precision_number as f64))).round()
                    / (10_f64).powf(precision_number as f64)),
                span,
            },
            None => Value::Int {
                val: val.round() as i64,
                span,
            },
        },
        Value::Error { .. } => value,
        other => Value::Error {
            error: Box::new(ShellError::OnlySupportsThisInputType {
                exp_input_type: "numeric".into(),
                wrong_type: other.get_type().to_string(),
                dst_span: head,
                src_span: other.expect_span(),
            }),
        },
    }
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
