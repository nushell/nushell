#![allow(unused_variables, unused_imports, dead_code, unused_mut)]
use chrono::{DateTime, FixedOffset};

use nu_cmd_base::input_handler::{operate, CmdArgument};
use nu_engine::CallExt;
use nu_protocol::{
    ast::{Call, CellPath},
    engine::{Command, EngineState, Stack},
    Category, Example, NuDuration, PipelineData, ShellError, Signature, Span, SyntaxShape, Type,
    Unit, Value,
};

struct Arguments {
    unit: NuDuration,
    date: Option<DateTime<FixedOffset>>,
    cell_paths: Option<Vec<CellPath>>,
}

impl CmdArgument for Arguments {
    fn take_cell_paths(&mut self) -> Option<Vec<CellPath>> {
        self.cell_paths.take()
    }
}

#[derive(Clone)]
pub struct SubCommand;

impl Command for SubCommand {
    fn name(&self) -> &str {
        "into duration"
    }

    fn signature(&self) -> Signature {
        Signature::build("into duration")
            .input_output_types(vec![
                (Type::String, Type::Duration),
                (Type::Int, Type::Duration),
                (Type::Duration, Type::Duration),
                (
                    Type::Record(vec![
                        ("quantity".into(), Type::Number),
                        ("unit".into(), Type::String),
                    ]),
                    Type::Duration,
                ),
            ])
            .vectorizes_over_list(true)
            .named(
                "unit",
                SyntaxShape::Duration,
                "time unit of result (default 0_nanoseconds)",
                Some('u'),
            )
            .named(
                "date",
                SyntaxShape::DateTime,
                "base date for conversions between days and months.  Only specify if you plan on doing this.",
                Some('d'),
            )
            .rest(
                "rest",
                SyntaxShape::CellPath,
                "for structured data input, convert data at the given cell path",
            )
            .category(Category::Conversions)
    }

    fn usage(&self) -> &str {
        "Convert input to duration."
    }

    fn search_terms(&self) -> Vec<&str> {
        vec!["convert", "time", "period"]
    }

    fn run(
        &self,
        engine_state: &EngineState,
        stack: &mut Stack,
        call: &Call,
        input: PipelineData,
    ) -> Result<PipelineData, ShellError> {
        let cell_paths = call.rest(engine_state, stack, 0)?;
        let cell_paths = (!cell_paths.is_empty()).then_some(cell_paths);

        let mut unit_arg = call.get_flag::<Value>(engine_state, stack, "unit")?;
        if unit_arg.is_none() {
            unit_arg = call.get_flag::<Value>(engine_state, stack, "units")?; // allow --units or --unit (maybe?)
        }
        let unit = match unit_arg {
            Some(Value::Duration { val, span }) => val,
            Some(_) | None => NuDuration::ns(0),
        };

        let mut date_arg = call.get_flag::<Value>(engine_state, stack, "date")?;
        let date = match date_arg {
            Some(Value::Date { val, span }) => Some(val),
            _ => None,
        };

        let args = Arguments {
            unit,
            date,
            cell_paths,
        };
        operate(action, args, input, call.head, engine_state.ctrlc.clone())
    }

    fn examples(&self) -> Vec<Example> {
        vec![]
    }
}

fn action(input: &Value, args: &Arguments, span: Span) -> Value {
    match input {
        Value::Int { val: _, .. } => Value::test_duration(NuDuration::ns(0)),
        Value::String { val, .. } => Value::test_duration(NuDuration::ns(0)),
        Value::Duration {
            val,
            span: val_span,
        } => Value::test_duration(NuDuration::ns(0)),
        Value::Binary { val, span } => Value::test_duration(NuDuration::ns(0)),
        // Propagate errors by explicitly matching them before the final case.
        Value::Error { .. } => input.clone(),
        other => Value::Error {
            error: Box::new(ShellError::OnlySupportsThisInputType {
                exp_input_type: "integer, float, filesize, date, string, binary, duration or bool"
                    .into(),
                wrong_type: other.get_type().to_string(),
                dst_span: span,
                src_span: other.expect_span(),
            }),
        },
    };
    Value::Nothing { span }
}

#[cfg(test)]
mod test {
    use chrono::{DateTime, FixedOffset};
    use rstest::rstest;

    use super::Value;
    use super::*;
    use nu_protocol::Type::Error;

    #[test]
    fn test_examples() {
        use crate::test_examples;

        //todo: reenable after implementing some examples for into duration
        //todo: test_examples(SubCommand {})
    }
}
