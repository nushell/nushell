use chrono::Local;
use nu_engine::CallExt;
use nu_protocol::{
    ast::Call,
    engine::{Command, EngineState, Stack},
    Category, Example, PipelineData, Signature, Span, Spanned, SyntaxShape, Value,
};

use super::utils::{parse_date_from_string, unsupported_input_error};

#[derive(Clone)]
pub struct SubCommand;

impl Command for SubCommand {
    fn name(&self) -> &str {
        "date format"
    }

    fn signature(&self) -> Signature {
        Signature::build("date format")
            .required(
                "format string",
                SyntaxShape::String,
                "the desired date format",
            )
            .category(Category::Date)
    }

    fn usage(&self) -> &str {
        "Format a given date using the given format string."
    }

    fn run(
        &self,
        engine_state: &EngineState,
        stack: &mut Stack,
        call: &Call,
        input: PipelineData,
    ) -> Result<nu_protocol::PipelineData, nu_protocol::ShellError> {
        let head = call.head;
        let formatter: Spanned<String> = call.req(engine_state, stack, 0)?;
        input.map(
            move |value| format_helper(value, &formatter, head),
            engine_state.ctrlc.clone(),
        )
    }

    fn examples(&self) -> Vec<Example> {
        vec![
            Example {
                description: "Format a given date using the given format string.",
                example: "date format '%Y-%m-%d'",
                result: Some(Value::String {
                    val: Local::now().format("%Y-%m-%d").to_string(),
                    span: Span::unknown(),
                }),
            },
            Example {
                description: "Format a given date using the given format string.",
                example: r#"date format "%Y-%m-%d %H:%M:%S""#,
                result: Some(Value::String {
                    val: Local::now().format("%Y-%m-%d %H:%M:%S").to_string(),
                    span: Span::unknown(),
                }),
            },
            Example {
                description: "Format a given date using the given format string.",
                example: r#""2021-10-22 20:00:12 +01:00" | date format "%Y-%m-%d""#,
                result: Some(Value::String {
                    val: "2021-10-22".into(),
                    span: Span::unknown(),
                }),
            },
        ]
    }
}

fn format_helper(value: Value, formatter: &Spanned<String>, span: Span) -> Value {
    match value {
        Value::Date { val, span: _ } => Value::String {
            val: val.format(formatter.item.as_str()).to_string(),
            span,
        },
        Value::String { val, span: _ } => {
            let dt = parse_date_from_string(val);
            match dt {
                Ok(x) => Value::String {
                    val: x.format(formatter.item.as_str()).to_string(),
                    span,
                },
                Err(e) => e,
            }
        }
        Value::Nothing { span: _ } => {
            let dt = Local::now();
            Value::String {
                val: dt
                    .with_timezone(dt.offset())
                    .format(formatter.item.as_str())
                    .to_string(),
                span,
            }
        }
        _ => unsupported_input_error(),
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
