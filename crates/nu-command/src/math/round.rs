use crate::math::utils::run_with_elementwise;
use nu_engine::command_prelude::*;

#[derive(Clone)]
pub struct MathRound;

impl Command for MathRound {
    fn name(&self) -> &str {
        "math round"
    }

    fn signature(&self) -> Signature {
        Signature::build("math round")
            .input_output_types(vec![
                (Type::Number, Type::Number),
                (Type::Duration, Type::Duration),
                (Type::Filesize, Type::Filesize),
                (
                    Type::List(Box::new(Type::Number)),
                    Type::List(Box::new(Type::Number)),
                ),
                (
                    Type::List(Box::new(Type::Duration)),
                    Type::List(Box::new(Type::Duration)),
                ),
                (
                    Type::List(Box::new(Type::Filesize)),
                    Type::List(Box::new(Type::Filesize)),
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
            .named(
                "precision",
                SyntaxShape::Number,
                "Digits of precision.",
                Some('p'),
            )
            .category(Category::Math)
    }

    fn description(&self) -> &str {
        "Returns the input number rounded to the specified precision."
    }

    fn search_terms(&self) -> Vec<&str> {
        vec!["approx", "closest", "nearest"]
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
        let precision_param: Option<i64> = call.get_flag(engine_state, stack, "precision")?;
        let cell_paths: Vec<CellPath> = call.rest(engine_state, stack, 0)?;
        let head = call.head;
        run_with_elementwise(
            input,
            cell_paths,
            head,
            engine_state.signals(),
            true,
            move |value| operate(value, head, precision_param),
        )
    }

    fn run_const(
        &self,
        working_set: &StateWorkingSet,
        call: &Call,
        input: PipelineData,
    ) -> Result<PipelineData, ShellError> {
        let precision_param: Option<i64> = call.get_flag_const(working_set, "precision")?;
        let cell_paths: Vec<CellPath> = call.rest_const(working_set, 0)?;
        let head = call.head;
        run_with_elementwise(
            input,
            cell_paths,
            head,
            working_set.permanent().signals(),
            true,
            move |value| operate(value, head, precision_param),
        )
    }

    fn examples(&self) -> Vec<Example<'_>> {
        vec![
            Example {
                description: "Apply the round function to a list of numbers.",
                example: "[1.5 2.3 -3.1] | math round",
                result: Some(Value::list(
                    vec![Value::test_int(2), Value::test_int(2), Value::test_int(-3)],
                    Span::test_data(),
                )),
            },
            Example {
                description: "Apply the round function with precision specified.",
                example: "[1.555 2.333 -3.111] | math round --precision 2",
                result: Some(Value::list(
                    vec![
                        Value::test_float(1.56),
                        Value::test_float(2.33),
                        Value::test_float(-3.11),
                    ],
                    Span::test_data(),
                )),
            },
            Example {
                description: "Apply negative precision to a list of numbers.",
                example: "[123, 123.3, -123.4] | math round --precision -1",
                result: Some(Value::list(
                    vec![
                        Value::test_int(120),
                        Value::test_int(120),
                        Value::test_int(-120),
                    ],
                    Span::test_data(),
                )),
            },
            Example {
                description: "Apply the round function to list-valued columns in a record.",
                example: "{alice: [1.2 2.7 3.5], bob: [4.1 5.9]} | math round",
                result: Some(Value::test_record(record! {
                    "alice" => Value::list(
                        vec![Value::test_int(1), Value::test_int(3), Value::test_int(4)],
                        Span::test_data(),
                    ),
                    "bob" => Value::list(
                        vec![Value::test_int(4), Value::test_int(6)],
                        Span::test_data(),
                    ),
                })),
            },
            Example {
                description: "Apply the round function to a single column using a cell path.",
                example: "{alice: [1.2 2.7 3.5], bob: [4.1 5.9]} | math round alice",
                result: Some(Value::test_record(record! {
                    "alice" => Value::list(
                        vec![Value::test_int(1), Value::test_int(3), Value::test_int(4)],
                        Span::test_data(),
                    ),
                    "bob" => Value::list(
                        vec![Value::test_float(4.1), Value::test_float(5.9)],
                        Span::test_data(),
                    ),
                })),
            },
        ]
    }
}

fn operate(value: Value, head: Span, precision: Option<i64>) -> Value {
    let span = value.span();

    // Duration and filesize are already integer units (ns / bytes). Identity is
    // correct without --precision; decimal precision is not meaningful for units.
    if matches!(value, Value::Duration { .. } | Value::Filesize { .. }) {
        if precision.is_some() {
            return Value::error(
                ShellError::UnsupportedInput {
                    msg: "'math round --precision' is not supported for duration or filesize"
                        .into(),
                    input: "value originates from here".into(),
                    msg_span: head,
                    input_span: span,
                },
                span,
            );
        }
        return value;
    }

    // We treat int values as float values to share the rounding path.
    let float_val = match &value {
        Value::Int { val, .. } => *val as f64,
        Value::Float { val, .. } => *val,
        Value::Error { .. } => return value,
        other => {
            return Value::error(
                ShellError::OnlySupportsThisInputType {
                    exp_input_type: crate::math::utils::NUMERIC_INPUT_TYPES.into(),
                    wrong_type: other.get_type().to_string(),
                    dst_span: head,
                    src_span: other.span(),
                },
                head,
            );
        }
    };

    if !float_val.is_finite() {
        return Value::error(
            ShellError::UnsupportedInput {
                msg: "cannot round non-finite number".into(),
                input: "value originates from here".into(),
                msg_span: span,
                input_span: span,
            },
            span,
        );
    }

    match precision {
        Some(precision_number) => Value::float(
            (float_val * ((10_f64).powf(precision_number as f64))).round()
                / (10_f64).powf(precision_number as f64),
            span,
        ),
        None => Value::int(float_val.round() as i64, span),
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test_examples() -> nu_test_support::Result {
        nu_test_support::test().examples(MathRound)
    }
}
