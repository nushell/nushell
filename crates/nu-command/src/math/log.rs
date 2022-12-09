use nu_engine::CallExt;
use nu_protocol::ast::Call;
use nu_protocol::engine::{Command, EngineState, Stack};
use nu_protocol::{
    Category, Example, PipelineData, ShellError, Signature, Span, Spanned, SyntaxShape, Type, Value,
};

#[derive(Clone)]
pub struct SubCommand;

impl Command for SubCommand {
    fn name(&self) -> &str {
        "math log"
    }

    fn signature(&self) -> Signature {
        Signature::build("math log")
            .required(
                "base",
                SyntaxShape::Number,
                "Base for which the logarithm should be computed",
            )
            .input_output_types(vec![(Type::Number, Type::Float)])
            .vectorizes_over_list(true)
            .category(Category::Math)
    }

    fn usage(&self) -> &str {
        "Returns the logarithm for an arbitrary base."
    }

    fn search_terms(&self) -> Vec<&str> {
        vec!["base", "exponent", "inverse", "euler"]
    }

    fn run(
        &self,
        engine_state: &EngineState,
        stack: &mut Stack,
        call: &Call,
        input: PipelineData,
    ) -> Result<nu_protocol::PipelineData, nu_protocol::ShellError> {
        let head = call.head;
        let base: Spanned<f64> = call.req(engine_state, stack, 0)?;

        if base.item <= 0.0f64 {
            return Err(ShellError::UnsupportedInput(
                "Base has to be greater 0".into(),
                base.span,
            ));
        }

        let base = base.item;
        input.map(
            move |value| operate(value, head, base),
            engine_state.ctrlc.clone(),
        )
    }

    fn examples(&self) -> Vec<Example> {
        vec![
            Example {
                description: "Get the logarithm of 100 to the base 10",
                example: "100 | math log 10",
                result: Some(Value::test_float(2.0f64)),
            },
            Example {
                example: "[16 8 4] | math log 2",
                description: "Get the log2 of a list of values",
                result: Some(Value::List {
                    vals: vec![
                        Value::test_float(4.0),
                        Value::test_float(3.0),
                        Value::test_float(2.0),
                    ],
                    span: Span::test_data(),
                }),
            },
        ]
    }
}

fn operate(value: Value, head: Span, base: f64) -> Value {
    match value {
        numeric @ (Value::Int { .. } | Value::Float { .. }) => {
            let (val, span) = match numeric {
                Value::Int { val, span } => (val as f64, span),
                Value::Float { val, span } => (val, span),
                _ => unreachable!(),
            };

            if val <= 0.0 {
                return Value::Error {
                    error: ShellError::UnsupportedInput(
                        "'math log' undefined for values outside the open interval (0, Inf)."
                            .into(),
                        span,
                    ),
                };
            }
            // Specialize for better precision/performance
            let val = if base == 10.0 {
                val.log10()
            } else if base == 2.0 {
                val.log2()
            } else {
                val.log(base)
            };

            Value::Float { val, span }
        }
        other => Value::Error {
            error: ShellError::UnsupportedInput(
                format!(
                    "Only numerical values are supported, input type: {:?}",
                    other.get_type()
                ),
                other.span().unwrap_or(head),
            ),
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
