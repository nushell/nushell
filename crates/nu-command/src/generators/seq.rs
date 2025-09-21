use nu_engine::command_prelude::*;
use nu_protocol::ListStream;

#[derive(Clone)]
pub struct Seq;

impl Command for Seq {
    fn name(&self) -> &str {
        "seq"
    }

    fn signature(&self) -> Signature {
        Signature::build("seq")
            .input_output_types(vec![(Type::Nothing, Type::List(Box::new(Type::Number)))])
            .rest("rest", SyntaxShape::Number, "Sequence values.")
            .category(Category::Generators)
    }

    fn description(&self) -> &str {
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

    fn examples(&self) -> Vec<Example<'_>> {
        vec![
            Example {
                description: "sequence 1 to 10",
                example: "seq 1 10",
                result: Some(Value::list(
                    vec![
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
                    Span::test_data(),
                )),
            },
            Example {
                description: "sequence 1.0 to 2.0 by 0.1s",
                example: "seq 1.0 0.1 2.0",
                result: Some(Value::list(
                    vec![
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
                    Span::test_data(),
                )),
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

    // note that the check for int or float has to occur here. prior, the check would occur after
    // everything had been generated; this does not work well with ListStreams.
    // As such, the simple test is to check if this errors out: that means there is a float in the
    // input, which necessarily means that parts of the output will be floats.
    let rest_nums_check: Result<Vec<Spanned<i64>>, ShellError> = call.rest(engine_state, stack, 0);
    let contains_decimals = rest_nums_check.is_err();

    if rest_nums.is_empty() {
        return Err(ShellError::GenericError {
            error: "seq requires some parameters".into(),
            msg: "needs parameter".into(),
            span: Some(call.head),
            help: None,
            inner: vec![],
        });
    }

    let rest_nums: Vec<f64> = rest_nums.iter().map(|n| n.item).collect();

    run_seq(rest_nums, span, contains_decimals, engine_state)
}

pub fn run_seq(
    free: Vec<f64>,
    span: Span,
    contains_decimals: bool,
    engine_state: &EngineState,
) -> Result<PipelineData, ShellError> {
    let first = free[0];
    let step = if free.len() > 2 { free[1] } else { 1.0 };
    let last = { free[free.len() - 1] };

    let stream = if !contains_decimals {
        ListStream::new(
            IntSeq {
                count: first as i64,
                step: step as i64,
                last: last as i64,
                span,
            },
            span,
            engine_state.signals().clone(),
        )
    } else {
        ListStream::new(
            FloatSeq {
                first,
                step,
                last,
                index: 0,
                span,
            },
            span,
            engine_state.signals().clone(),
        )
    };

    Ok(stream.into())
}

struct FloatSeq {
    first: f64,
    step: f64,
    last: f64,
    index: isize,
    span: Span,
}

impl Iterator for FloatSeq {
    type Item = Value;
    fn next(&mut self) -> Option<Value> {
        let count = self.first + self.index as f64 * self.step;
        // Accuracy guaranteed as far as possible; each time, the value is re-evaluated from the
        // base arguments
        if (count > self.last && self.step >= 0.0) || (count < self.last && self.step <= 0.0) {
            return None;
        }
        self.index += 1;
        Some(Value::float(count, self.span))
    }
}

struct IntSeq {
    count: i64,
    step: i64,
    last: i64,
    span: Span,
}

impl Iterator for IntSeq {
    type Item = Value;
    fn next(&mut self) -> Option<Value> {
        if (self.count > self.last && self.step >= 0) || (self.count < self.last && self.step <= 0)
        {
            return None;
        }
        let ret = Some(Value::int(self.count, self.span));
        self.count += self.step;
        ret
    }
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
