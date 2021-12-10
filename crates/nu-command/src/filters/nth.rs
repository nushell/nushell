use nu_engine::CallExt;
use nu_protocol::ast::Call;
use nu_protocol::engine::{Command, EngineState, Stack};
use nu_protocol::{
    Category, Example, IntoInterruptiblePipelineData, PipelineData, PipelineIterator, ShellError,
    Signature, Span, SyntaxShape, Value,
};

#[derive(Clone)]
pub struct Nth;

impl Command for Nth {
    fn name(&self) -> &str {
        "nth"
    }

    fn signature(&self) -> Signature {
        Signature::build("nth")
            .rest("rest", SyntaxShape::Int, "the number of the row to return")
            .switch("skip", "Skip the rows instead of selecting them", Some('s'))
            .category(Category::Filters)
    }

    fn usage(&self) -> &str {
        "Return or skip only the selected rows."
    }

    fn examples(&self) -> Vec<Example> {
        vec![
            Example {
                example: "[sam,sarah,2,3,4,5] | nth 0 1 2",
                description: "Get the first, second, and third row",
                result: Some(Value::List {
                    vals: vec![
                        Value::test_string("sam"),
                        Value::test_string("sarah"),
                        Value::test_int(2),
                    ],
                    span: Span::unknown(),
                }),
            },
            Example {
                example: "[0,1,2,3,4,5] | nth 0 1 2",
                description: "Get the first, second, and third row",
                result: Some(Value::List {
                    vals: vec![Value::test_int(0), Value::test_int(1), Value::test_int(2)],
                    span: Span::unknown(),
                }),
            },
            Example {
                example: "[0,1,2,3,4,5] | nth -s 0 1 2",
                description: "Skip the first, second, and third row",
                result: Some(Value::List {
                    vals: vec![Value::test_int(3), Value::test_int(4), Value::test_int(5)],
                    span: Span::unknown(),
                }),
            },
            Example {
                example: "[0,1,2,3,4,5] | nth 0 2 4",
                description: "Get the first, third, and fifth row",
                result: Some(Value::List {
                    vals: vec![Value::test_int(0), Value::test_int(2), Value::test_int(4)],
                    span: Span::unknown(),
                }),
            },
            Example {
                example: "[0,1,2,3,4,5] | nth 2 0 4",
                description: "Get the first, third, and fifth row",
                result: Some(Value::List {
                    vals: vec![Value::test_int(0), Value::test_int(2), Value::test_int(4)],
                    span: Span::unknown(),
                }),
            },
        ]
    }

    fn run(
        &self,
        engine_state: &EngineState,
        stack: &mut Stack,
        call: &Call,
        input: PipelineData,
    ) -> Result<PipelineData, ShellError> {
        let mut rows: Vec<usize> = call.rest(engine_state, stack, 0)?;
        rows.sort_unstable();
        let skip = call.has_flag("skip");
        let pipeline_iter: PipelineIterator = input.into_iter();

        Ok(NthIterator {
            input: pipeline_iter,
            rows,
            skip,
            current: 0,
        }
        .into_pipeline_data(engine_state.ctrlc.clone()))
    }
}

struct NthIterator {
    input: PipelineIterator,
    rows: Vec<usize>,
    skip: bool,
    current: usize,
}

impl Iterator for NthIterator {
    type Item = Value;

    fn next(&mut self) -> Option<Self::Item> {
        loop {
            if !self.skip {
                if let Some(row) = self.rows.get(0) {
                    if self.current == *row {
                        self.rows.remove(0);
                        self.current += 1;
                        return self.input.next();
                    } else {
                        self.current += 1;
                        let _ = self.input.next();
                        continue;
                    }
                } else {
                    return None;
                }
            } else if let Some(row) = self.rows.get(0) {
                if self.current == *row {
                    self.rows.remove(0);
                    self.current += 1;
                    let _ = self.input.next();
                    continue;
                } else {
                    self.current += 1;
                    return self.input.next();
                }
            } else {
                return self.input.next();
            }
        }
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test_examples() {
        use crate::test_examples;

        test_examples(Nth {})
    }
}
