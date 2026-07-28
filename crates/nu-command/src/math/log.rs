use crate::math::utils::run_with_elementwise;
use nu_engine::command_prelude::*;

#[derive(Clone)]
pub struct MathLog;

impl Command for MathLog {
    fn name(&self) -> &str {
        "math log"
    }

    fn signature(&self) -> Signature {
        Signature::build("math log")
            .required(
                "base",
                SyntaxShape::Number,
                "Base for which the logarithm should be computed.",
            )
            .input_output_types(vec![
                (Type::Number, Type::Float),
                (
                    Type::List(Box::new(Type::Number)),
                    Type::List(Box::new(Type::Float)),
                ),
                (Type::Range, Type::List(Box::new(Type::Number))),
                (Type::record(), Type::record()),
            ])
            .rest(
                "columns",
                SyntaxShape::CellPath,
                "The cell-paths/columns to operate on.",
            )
            .allow_variants_without_examples(true)
            .category(Category::Math)
    }

    fn description(&self) -> &str {
        "Returns the logarithm for an arbitrary base."
    }

    fn search_terms(&self) -> Vec<&str> {
        vec!["base", "exponent", "inverse", "euler"]
    }

    fn is_const(&self) -> bool {
        true
    }

    fn run(
        &self,
        engine_state: &EngineState,
        stack: &mut Stack,
        call: &Call,
        input: PipelineData,
    ) -> Result<PipelineData, ShellError> {
        let base = require_positive_base(call.req(engine_state, stack, 0)?, call.head)?;
        let cell_paths: Vec<CellPath> = call.rest(engine_state, stack, 1)?;
        let head = call.head;
        run_with_elementwise(
            input,
            cell_paths,
            head,
            engine_state.signals(),
            true,
            move |value| operate(value, head, base),
        )
    }

    fn run_const(
        &self,
        working_set: &StateWorkingSet,
        call: &Call,
        input: PipelineData,
    ) -> Result<PipelineData, ShellError> {
        let base = require_positive_base(call.req_const(working_set, 0)?, call.head)?;
        let cell_paths: Vec<CellPath> = call.rest_const(working_set, 1)?;
        let head = call.head;
        run_with_elementwise(
            input,
            cell_paths,
            head,
            working_set.permanent().signals(),
            true,
            move |value| operate(value, head, base),
        )
    }

    fn examples(&self) -> Vec<Example<'_>> {
        vec![
            Example {
                description: "Get the logarithm of 100 to the base 10.",
                example: "100 | math log 10",
                result: Some(Value::test_float(2.0f64)),
            },
            Example {
                example: "[16 8 4] | math log 2",
                description: "Get the log2 of a list of values.",
                result: Some(Value::list(
                    vec![
                        Value::test_float(4.0),
                        Value::test_float(3.0),
                        Value::test_float(2.0),
                    ],
                    Span::test_data(),
                )),
            },
            Example {
                description: "Compute the log base 10 of list-valued columns in a record.",
                example: "{alice: [1 10 100], bob: [1000 10000]} | math log 10",
                result: Some(Value::test_record(record! {
                    "alice" => Value::list(
                        vec![Value::test_float(0.0), Value::test_float(1.0), Value::test_float(2.0)],
                        Span::test_data(),
                    ),
                    "bob" => Value::list(
                        vec![Value::test_float(3.0), Value::test_float(4.0)],
                        Span::test_data(),
                    ),
                })),
            },
            Example {
                description: "Compute the log base 10 of a single column using a cell path.",
                example: "{alice: [1 10 100], bob: [1000 10000]} | math log 10 alice",
                result: Some(Value::test_record(record! {
                    "alice" => Value::list(
                        vec![Value::test_float(0.0), Value::test_float(1.0), Value::test_float(2.0)],
                        Span::test_data(),
                    ),
                    "bob" => Value::list(
                        vec![Value::test_float(1000.0), Value::test_float(10000.0)],
                        Span::test_data(),
                    ),
                })),
            },
        ]
    }
}

fn require_positive_base(base: Spanned<f64>, head: Span) -> Result<f64, ShellError> {
    if base.item <= 0.0f64 {
        return Err(ShellError::UnsupportedInput {
            msg: "Base has to be greater 0".into(),
            input: "value originates from here".into(),
            msg_span: head,
            input_span: base.span,
        });
    }
    Ok(base.item)
}

fn operate(value: Value, head: Span, base: f64) -> Value {
    let span = value.span();
    match value {
        numeric @ (Value::Int { .. } | Value::Float { .. }) => {
            let (val, span) = match numeric {
                Value::Int { val, .. } => (val as f64, span),
                Value::Float { val, .. } => (val, span),
                _ => unreachable!(),
            };

            if val <= 0.0 {
                return Value::error(
                    ShellError::UnsupportedInput {
                        msg: "'math log' undefined for values outside the open interval (0, Inf)."
                            .into(),
                        input: "value originates from here".into(),
                        msg_span: head,
                        input_span: span,
                    },
                    span,
                );
            }
            // Specialize for better precision/performance
            let val = if base == 10.0 {
                val.log10()
            } else if base == 2.0 {
                val.log2()
            } else {
                val.log(base)
            };

            Value::float(val, span)
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

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test_examples() -> nu_test_support::Result {
        nu_test_support::test().examples(MathLog)
    }
}
