use chrono::{DateTime, Utc};
use nu_engine::command_prelude::*;
use nu_protocol::shell_error::generic::GenericError;

#[derive(Clone)]
pub struct DateFloor;

impl Command for DateFloor {
    fn name(&self) -> &str {
        "date floor"
    }

    fn signature(&self) -> Signature {
        Signature::build("date floor")
            .input_output_types(vec![
                (Type::Date, Type::Date),
                (
                    Type::List(Box::new(Type::Date)),
                    Type::List(Box::new(Type::Date)),
                ),
            ])
            .required(
                "period",
                SyntaxShape::Duration,
                "Duration value representing the period boundary to which to round down to.",
            )
            .category(Category::Date)
    }

    fn description(&self) -> &str {
        "Round to the nearest specified datetime boundary before the input date."
    }

    fn run(
        &self,
        engine_state: &EngineState,
        stack: &mut Stack,
        call: &Call,
        input: PipelineData,
    ) -> Result<PipelineData, ShellError> {
        let duration_ns = match call.req::<Value>(engine_state, stack, 0)? {
            Value::Duration { val, .. } => Ok(val),
            x => Err(ShellError::IncompatibleParametersSingle {
                msg: format!("Expected duration type but got {}", x.get_type()),
                span: call.head,
            }),
        }?;

        if let PipelineData::Empty = input {
            return Err(ShellError::PipelineEmpty {
                dst_span: call.head,
            });
        }

        input.map(
            move |value| match value {
                Value::Date {
                    val, internal_span, ..
                } => val
                    .timestamp_nanos_opt()
                    .map(|nanos| {
                        // use local-utc offset to properly adjust the day boundaries from UTC to input timezone
                        let offset_ns = val.timezone().local_minus_utc() as i64 * 1_000_000_000;
                        Value::date(
                            DateTime::<Utc>::from_timestamp_nanos(
                                nanos - ((nanos + offset_ns) % duration_ns),
                            )
                            .with_timezone(&val.timezone()),
                            internal_span,
                        )
                    })
                    .unwrap_or(Value::error(
                        ShellError::Generic(GenericError::new(
                            "date out of range",
                            "converting date to nanoseconds caused overflow",
                            internal_span,
                        )),
                        internal_span,
                    )),
                _ => Value::error(
                    ShellError::TypeMismatch {
                        err_message: "Input must be of type date".into(),
                        span: value.span(),
                    },
                    value.span(),
                ),
            },
            engine_state.signals(),
        )
    }

    fn examples(&self) -> Vec<Example<'_>> {
        vec![
            Example {
                description: "Round down to the nearest hour",
                example: "2026-07-15T12:11:10-04:00 | date floor 1hr",
                result: Some(Value::date(
                    DateTime::parse_from_str("2026-07-15 12:00:00 -0400", "%Y-%m-%d %H:%M:%S %z")
                        .expect("date calculation should not fail in test"),
                    Span::test_data(),
                )),
            },
            Example {
                description: "Round list of dates down to the nearest 2day boundary",
                example: "[2026-07-10T00:00:00-04:00 2026-07-15T00:00:00-04:00] | date floor 2day",
                result: Some(Value::list(
                    vec![
                        Value::date(
                            DateTime::parse_from_str(
                                "2026-07-10 00:00:00 -0400",
                                "%Y-%m-%d %H:%M:%S %z",
                            )
                            .expect("date calculation should not fail in test"),
                            Span::test_data(),
                        ),
                        Value::date(
                            DateTime::parse_from_str(
                                "2026-07-14 00:00:00 -0400",
                                "%Y-%m-%d %H:%M:%S %z",
                            )
                            .expect("date calculation should not fail in test"),
                            Span::test_data(),
                        ),
                    ],
                    Span::test_data(),
                )),
            },
        ]
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test_examples() -> nu_test_support::Result {
        nu_test_support::test().examples(DateFloor)
    }
}
