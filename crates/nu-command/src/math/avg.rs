use crate::math::{
    reducers::{Reduce, reducer_for},
    utils::run_with_function,
};
use nu_engine::command_prelude::*;

const NS_PER_SEC: i64 = 1_000_000_000;
#[derive(Clone)]
pub struct MathAvg;

impl Command for MathAvg {
    fn name(&self) -> &str {
        "math avg"
    }

    fn signature(&self) -> Signature {
        Signature::build("math avg")
            .input_output_types(vec![
                (Type::List(Box::new(Type::Duration)), Type::Duration),
                (Type::Duration, Type::Duration),
                (Type::List(Box::new(Type::Filesize)), Type::Filesize),
                (Type::Filesize, Type::Filesize),
                (Type::List(Box::new(Type::Number)), Type::Number),
                (Type::Number, Type::Number),
                (Type::Range, Type::Number),
                (Type::table(), Type::record()),
                (Type::record(), Type::record()),
            ])
            .allow_variants_without_examples(true)
            .category(Category::Math)
    }

    fn description(&self) -> &str {
        "Returns the average of a list of numbers."
    }

    fn search_terms(&self) -> Vec<&str> {
        vec!["average", "mean", "statistics"]
    }

    fn is_const(&self) -> bool {
        true
    }

    fn run(
        &self,
        _engine_state: &EngineState,
        _stack: &mut Stack,
        call: &Call,
        input: PipelineData,
    ) -> Result<PipelineData, ShellError> {
        run_with_function(call, input, average)
    }

    fn run_const(
        &self,
        _working_set: &StateWorkingSet,
        call: &Call,
        input: PipelineData,
    ) -> Result<PipelineData, ShellError> {
        run_with_function(call, input, average)
    }

    fn examples(&self) -> Vec<Example<'_>> {
        vec![
            Example {
                description: "Compute the average of a list of numbers",
                example: "[-50 100.0 25] | math avg",
                result: Some(Value::test_float(25.0)),
            },
            Example {
                description: "Compute the average of a list of durations",
                example: "[2sec 1min] | math avg",
                result: Some(Value::test_duration(31 * NS_PER_SEC)),
            },
            Example {
                description: "Compute the average of each column in a table",
                example: "[[a b]; [1 2] [3 4]] | math avg",
                result: Some(Value::test_record(record! {
                    "a" => Value::test_int(2),
                    "b" => Value::test_int(3),
                })),
            },
        ]
    }
}

pub fn average(values: &[Value], span: Span, head: Span) -> Result<Value, ShellError> {
    let sum = reducer_for(Reduce::Summation);
    let total = &sum(Value::int(0, head), values.to_vec(), span, head)?;
    let span = total.span();
    match total {
        Value::Filesize { val, .. } => Ok(Value::filesize(val.get() / values.len() as i64, span)),
        Value::Duration { val, .. } => Ok(Value::duration(val / values.len() as i64, span)),
        _ => total.div(head, &Value::int(values.len() as i64, head), head),
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test_examples() {
        use crate::test_examples;

        test_examples(MathAvg {})
    }
}
