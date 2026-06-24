use nu_engine::command_prelude::*;
use nu_protocol::{ListStream, Signals};

#[derive(Clone)]
pub struct Permutations;

impl Command for Permutations {
    fn name(&self) -> &str {
        "permutations"
    }

    fn signature(&self) -> Signature {
        Signature::build("permutations")
            .input_output_types(vec![
                (
                    Type::List(Box::new(Type::Any)),
                    Type::List(Box::new(Type::List(Box::new(Type::Any)))),
                ),
                (Type::table(), Type::table()),
            ])
            .category(Category::Filters)
    }

    fn description(&self) -> &str {
        "Generates all permutations of the input list."
    }

    fn search_terms(&self) -> Vec<&str> {
        vec!["arrangements", "orderings", "combinatorics", "factorial"]
    }

    fn run(
        &self,
        engine_state: &EngineState,
        _stack: &mut Stack,
        call: &Call,
        mut input: PipelineData,
    ) -> Result<PipelineData, ShellError> {
        let head = call.head;
        let metadata = input.take_metadata();

        let vals: Vec<Value> = input.into_iter().collect();
        let signals = engine_state.signals().clone();

        if vals.is_empty() {
            return Ok(PipelineData::Value(Value::list(vec![], head), metadata));
        }

        let iter = PermutationsIter::new(vals, signals.clone(), head);
        let stream = ListStream::new(iter, head, signals);
        Ok(PipelineData::ListStream(stream, metadata))
    }

    fn examples(&self) -> Vec<Example<'static>> {
        vec![
            Example {
                example: "[1 2 3] | permutations",
                description: "Generate all permutations",
                result: Some(Value::test_list(vec![
                    Value::test_list(vec![
                        Value::test_int(1),
                        Value::test_int(2),
                        Value::test_int(3),
                    ]),
                    Value::test_list(vec![
                        Value::test_int(2),
                        Value::test_int(1),
                        Value::test_int(3),
                    ]),
                    Value::test_list(vec![
                        Value::test_int(3),
                        Value::test_int(1),
                        Value::test_int(2),
                    ]),
                    Value::test_list(vec![
                        Value::test_int(1),
                        Value::test_int(3),
                        Value::test_int(2),
                    ]),
                    Value::test_list(vec![
                        Value::test_int(2),
                        Value::test_int(3),
                        Value::test_int(1),
                    ]),
                    Value::test_list(vec![
                        Value::test_int(3),
                        Value::test_int(2),
                        Value::test_int(1),
                    ]),
                ])),
            },
            Example {
                example: "[1 2] | permutations",
                description: "Generate permutations of two elements",
                result: Some(Value::test_list(vec![
                    Value::test_list(vec![Value::test_int(1), Value::test_int(2)]),
                    Value::test_list(vec![Value::test_int(2), Value::test_int(1)]),
                ])),
            },
            Example {
                example: "[] | permutations",
                description: "Empty list yields no permutations",
                result: Some(Value::test_list(vec![])),
            },
        ]
    }
}

/// A streaming iterator that yields all permutations of a list using Heap's algorithm.
///
/// Heap's algorithm generates each permutation by swapping a single pair of
/// elements from the previous permutation, making it well-suited for streaming
/// (O(1) amortized per permutation). The `c` array tracks the current state
/// of the nested loop counters.
struct PermutationsIter {
    values: Vec<Value>,
    c: Vec<usize>,
    i: usize,
    n: usize,
    first: bool,
    done: bool,
    signals: Signals,
    span: Span,
}

impl PermutationsIter {
    fn new(values: Vec<Value>, signals: Signals, span: Span) -> Self {
        let n = values.len();
        let done = n == 0;
        Self {
            c: vec![0; n],
            i: 0,
            n,
            values,
            first: true,
            done,
            signals,
            span,
        }
    }
}

impl Iterator for PermutationsIter {
    type Item = Value;

    fn next(&mut self) -> Option<Self::Item> {
        if self.done {
            return None;
        }

        if self.signals.interrupted() {
            return None;
        }

        if self.first {
            self.first = false;
            return Some(Value::list(self.values.clone(), self.span));
        }

        // Heap's algorithm: generate the next permutation by swapping.
        while self.i < self.n {
            if self.c[self.i] < self.i {
                if self.i.is_multiple_of(2) {
                    self.values.swap(0, self.i);
                } else {
                    self.values.swap(self.c[self.i], self.i);
                }
                self.c[self.i] += 1;
                self.i = 0;
                return Some(Value::list(self.values.clone(), self.span));
            } else {
                self.c[self.i] = 0;
                self.i += 1;
            }
        }

        self.done = true;
        None
    }
}

#[cfg(test)]
mod test {
    use super::Permutations;
    use nu_test_support::prelude::*;

    #[test]
    fn test_examples() -> nu_test_support::Result {
        nu_test_support::test().examples(Permutations)
    }

    #[test]
    fn permutations_n3() -> Result {
        let result: Value = test().run("[1 2 3] | permutations")?;
        assert_eq!(
            result,
            Value::test_list(vec![
                Value::test_list(vec![
                    Value::test_int(1),
                    Value::test_int(2),
                    Value::test_int(3)
                ]),
                Value::test_list(vec![
                    Value::test_int(2),
                    Value::test_int(1),
                    Value::test_int(3)
                ]),
                Value::test_list(vec![
                    Value::test_int(3),
                    Value::test_int(1),
                    Value::test_int(2)
                ]),
                Value::test_list(vec![
                    Value::test_int(1),
                    Value::test_int(3),
                    Value::test_int(2)
                ]),
                Value::test_list(vec![
                    Value::test_int(2),
                    Value::test_int(3),
                    Value::test_int(1)
                ]),
                Value::test_list(vec![
                    Value::test_int(3),
                    Value::test_int(2),
                    Value::test_int(1)
                ]),
            ])
        );
        Ok(())
    }

    #[test]
    fn permutations_n2() -> Result {
        let result: Value = test().run("[1 2] | permutations")?;
        assert_eq!(
            result,
            Value::test_list(vec![
                Value::test_list(vec![Value::test_int(1), Value::test_int(2)]),
                Value::test_list(vec![Value::test_int(2), Value::test_int(1)]),
            ])
        );
        Ok(())
    }

    #[test]
    fn permutations_n1() -> Result {
        let result: Value = test().run("[42] | permutations")?;
        assert_eq!(
            result,
            Value::test_list(vec![Value::test_list(vec![Value::test_int(42)])])
        );
        Ok(())
    }

    #[test]
    fn permutations_empty() -> Result {
        test()
            .run("[] | permutations")
            .expect_value_eq(Value::test_list(vec![]))
    }

    #[test]
    fn permutations_n4_count() -> Result {
        // 4! = 24 permutations
        let result: Value = test().run("[1 2 3 4] | permutations | length")?;
        assert_eq!(result, Value::test_int(24));
        Ok(())
    }

    #[test]
    fn permutations_strings() -> Result {
        let result: Value = test().run("[a b] | permutations | get 0 | str join '-'")?;
        assert_eq!(result, Value::test_string("a-b"));
        Ok(())
    }
}
