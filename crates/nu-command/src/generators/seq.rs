use nu_engine::CallExt;
use nu_protocol::{
    ast::Call,
    engine::{Command, EngineState, Stack},
    Category, Example, IntoPipelineData, PipelineData, ShellError, Signature, Span, Spanned,
    SyntaxShape, Type, Value,
};

#[derive(Clone)]
pub struct Seq;

impl Command for Seq {
    fn name(&self) -> &str {
        "seq"
    }

    fn signature(&self) -> Signature {
        Signature::build("seq")
            .input_output_types(vec![(Type::Nothing, Type::List(Box::new(Type::Number)))])
            .rest("rest", SyntaxShape::Number, "sequence values")
            .category(Category::Generators)
    }

    fn usage(&self) -> &str {
        "Output sequences of numbers."
    }

    fn run(
        &self,
        engine_state: &EngineState,
        stack: &mut Stack,
        call: &Call,
        _input: PipelineData,
    ) -> Result<PipelineData, ShellError> {
        seq(engine_state, stack, call)
    }

    fn examples(&self) -> Vec<Example> {
        vec![
            Example {
                description: "sequence 1 to 10",
                example: "seq 1 10",
                result: Some(Value::List {
                    vals: vec![
                        Value::test_int(1),
                        Value::test_int(2),
                        Value::test_int(3),
                        Value::test_int(4),
                        Value::test_int(5),
                        Value::test_int(6),
                        Value::test_int(7),
                        Value::test_int(8),
                        Value::test_int(9),
                        Value::test_int(10),
                    ],
                    span: Span::test_data(),
                }),
            },
            Example {
                description: "sequence 1.0 to 2.0 by 0.1s",
                example: "seq 1.0 0.1 2.0",
                result: Some(Value::List {
                    vals: vec![
                        Value::test_float(1.0000),
                        Value::test_float(1.1000),
                        Value::test_float(1.2000),
                        Value::test_float(1.3000),
                        Value::test_float(1.4000),
                        Value::test_float(1.5000),
                        Value::test_float(1.6000),
                        Value::test_float(1.7000),
                        Value::test_float(1.8000),
                        Value::test_float(1.9000),
                        Value::test_float(2.0000),
                    ],
                    span: Span::test_data(),
                }),
            },
            Example {
                description: "sequence 1 to 5, then convert to a string with a pipe separator",
                example: "seq 1 5 | str join '|'",
                result: None,
            },
        ]
    }
}

fn seq(
    engine_state: &EngineState,
    stack: &mut Stack,
    call: &Call,
) -> Result<PipelineData, ShellError> {
    let span = call.head;
    let rest_nums: Vec<Spanned<f64>> = call.rest(engine_state, stack, 0)?;

    if rest_nums.is_empty() {
        return Err(ShellError::GenericError(
            "seq requires some parameters".into(),
            "needs parameter".into(),
            Some(call.head),
            None,
            Vec::new(),
        ));
    }

    let rest_nums: Vec<f64> = rest_nums.iter().map(|n| n.item).collect();

    run_seq(rest_nums, span)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_examples() {
        use crate::test_examples;

        test_examples(Seq {})
    }
}

pub fn run_seq(free: Vec<f64>, span: Span) -> Result<PipelineData, ShellError> {
    let first = free[0];

    let step: f64 = if free.len() > 2 { free[1] } else { 1.0 };
    let last = { free[free.len() - 1] };

    Ok(print_seq(first, step, last, span))
}

fn done_printing(next: f64, step: f64, last: f64) -> bool {
    if step >= 0f64 {
        next > last
    } else {
        next < last
    }
}

fn print_seq(first: f64, step: f64, last: f64, span: Span) -> PipelineData {
    let mut i = 0isize;
    let mut value = first + i as f64 * step;
    let mut ret_num = vec![];

    while !done_printing(value, step, last) {
        ret_num.push(value);
        i += 1;
        value = first + i as f64 * step;
    }

    // we'd like to keep the datatype the same for the output, so check
    // and see if any of the output contains values after the decimal point,
    // and if so we'll make the entire output floats
    let contains_decimals = vec_contains_decimals(&ret_num);
    let rows: Vec<Value> = ret_num
        .iter()
        .map(|v| {
            if contains_decimals {
                Value::float(*v, span)
            } else {
                Value::int(*v as i64, span)
            }
        })
        .collect();

    Value::List { vals: rows, span }.into_pipeline_data()
}

fn vec_contains_decimals(array: &[f64]) -> bool {
    let mut found_decimal = false;
    for x in array {
        if x.fract() != 0.0 {
            found_decimal = true;
            break;
        }
    }

    found_decimal
}
