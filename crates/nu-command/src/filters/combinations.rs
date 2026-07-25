use nu_engine::command_prelude::*;
use nu_protocol::{ListStream, Signals};

#[derive(Clone)]
pub struct Combinations;

impl Command for Combinations {
    fn name(&self) -> &str {
        "combinations"
    }

    fn signature(&self) -> Signature {
        Signature::build("combinations")
            .input_output_types(vec![
                (
                    Type::List(Box::new(Type::Any)),
                    Type::List(Box::new(Type::List(Box::new(Type::Any)))),
                ),
                (Type::table(), Type::table()),
            ])
            .required("k", SyntaxShape::Int, "The size of each combination (k).")
            .category(Category::Filters)
    }

    fn description(&self) -> &str {
        "Generates all combinations of size k from the input list."
    }

    fn search_terms(&self) -> Vec<&str> {
        vec!["choose", "subset", "combinatorics"]
    }

    fn run(
        &self,
        engine_state: &EngineState,
        stack: &mut Stack,
        call: &Call,
        mut input: PipelineData,
    ) -> Result<PipelineData, ShellError> {
        let head = call.head;
        let k: usize = call.req(engine_state, stack, 0)?;
        let metadata = input.take_metadata();

        let vals: Vec<Value> = input.into_iter().collect();
        let signals = engine_state.signals().clone();

        if k == 0 {
            // C(n, 0) = 1: the empty combination.
            return Ok(PipelineData::Value(
                Value::list(vec![Value::list(vec![], head)], head),
                metadata,
            ));
        }

        let iter = CombinationsIter::new(vals, k, signals.clone(), head);
        let stream = ListStream::new(iter, head, signals);
        Ok(PipelineData::ListStream(stream, metadata))
    }

    fn examples(&self) -> Vec<Example<'static>> {
        vec![
            Example {
                example: "[1 2 3] | combinations 2",
                description: "Generate all 2-combinations",
                result: Some(Value::test_list(vec![
                    Value::test_list(vec![Value::test_int(1), Value::test_int(2)]),
                    Value::test_list(vec![Value::test_int(1), Value::test_int(3)]),
                    Value::test_list(vec![Value::test_int(2), Value::test_int(3)]),
                ])),
            },
            Example {
                example: "[[a] [b] [c]] | combinations 2",
                description: "Generate combinations of lists",
                result: Some(Value::test_list(vec![
                    Value::test_list(vec![
                        Value::test_list(vec![Value::test_string("a")]),
                        Value::test_list(vec![Value::test_string("b")]),
                    ]),
                    Value::test_list(vec![
                        Value::test_list(vec![Value::test_string("a")]),
                        Value::test_list(vec![Value::test_string("c")]),
                    ]),
                    Value::test_list(vec![
                        Value::test_list(vec![Value::test_string("b")]),
                        Value::test_list(vec![Value::test_string("c")]),
                    ]),
                ])),
            },
            Example {
                example: "[1 2] | combinations 3",
                description: "k > n yields an empty list",
                result: Some(Value::test_list(vec![])),
            },
        ]
    }
}

/// A streaming iterator that yields all k-combinations from a list.
///
/// Combinations are produced in lexicographic order of their element indices.
/// The algorithm advances a vector of indices from the rightmost position,
/// incrementing the first index that can be increased without exceeding
/// `n - (k - i)`. This is the classic combinatorial number system.
struct CombinationsIter {
    n: usize,
    k: usize,
    indices: Vec<usize>,
    values: Vec<Value>,
    done: bool,
    signals: Signals,
    span: Span,
}

impl CombinationsIter {
    fn new(values: Vec<Value>, k: usize, signals: Signals, span: Span) -> Self {
        let n = values.len();
        let done = k == 0 || k > n;
        let indices: Vec<usize> = if done { vec![] } else { (0..k).collect() };
        Self {
            n,
            k,
            indices,
            values,
            done,
            signals,
            span,
        }
    }
}

impl Iterator for CombinationsIter {
    type Item = Value;

    fn next(&mut self) -> Option<Self::Item> {
        if self.done {
            return None;
        }

        if self.signals.interrupted() {
            return None;
        }

        let combination: Vec<Value> = self
            .indices
            .iter()
            .map(|&i| self.values[i].clone())
            .collect();
        let result = Some(Value::list(combination, self.span));

        // Advance to the next combination in lexicographic order.
        let mut i = self.k.wrapping_sub(1);
        while i < self.k && self.indices[i] == self.n - self.k + i {
            if i == 0 {
                self.done = true;
                return result;
            }
            i -= 1;
        }

        self.indices[i] += 1;
        for j in (i + 1)..self.k {
            self.indices[j] = self.indices[j - 1] + 1;
        }

        result
    }
}

#[cfg(test)]
mod test {
    use super::Combinations;
    use nu_test_support::prelude::*;

    #[test]
    fn test_examples() -> nu_test_support::Result {
        nu_test_support::test().examples(Combinations)
    }

    #[test]
    fn combinations_k2() -> Result {
        let result: Value = test().run("[1 2 3] | combinations 2")?;
        assert_eq!(
            result,
            Value::test_list(vec![
                Value::test_list(vec![Value::test_int(1), Value::test_int(2)]),
                Value::test_list(vec![Value::test_int(1), Value::test_int(3)]),
                Value::test_list(vec![Value::test_int(2), Value::test_int(3)]),
            ])
        );
        Ok(())
    }

    #[test]
    fn combinations_k0() -> Result {
        let result: Value = test().run("[1 2 3] | combinations 0")?;
        assert_eq!(result, Value::test_list(vec![Value::test_list(vec![])]));
        Ok(())
    }

    #[test]
    fn combinations_k1() -> Result {
        let result: Value = test().run("[1 2 3] | combinations 1")?;
        assert_eq!(
            result,
            Value::test_list(vec![
                Value::test_list(vec![Value::test_int(1)]),
                Value::test_list(vec![Value::test_int(2)]),
                Value::test_list(vec![Value::test_int(3)]),
            ])
        );
        Ok(())
    }

    #[test]
    fn combinations_k_equals_n() -> Result {
        let result: Value = test().run("[1 2 3] | combinations 3")?;
        assert_eq!(
            result,
            Value::test_list(vec![Value::test_list(vec![
                Value::test_int(1),
                Value::test_int(2),
                Value::test_int(3),
            ])])
        );
        Ok(())
    }

    #[test]
    fn combinations_k_greater_than_n() -> Result {
        test()
            .run("[1 2] | combinations 3")
            .expect_value_eq(Value::test_list(vec![]))
    }

    #[test]
    fn combinations_empty_list() -> Result {
        test()
            .run("[] | combinations 2")
            .expect_value_eq(Value::test_list(vec![]))
    }

    #[test]
    fn combinations_strings() -> Result {
        let result: Value = test().run("[a b c] | combinations 2")?;
        assert_eq!(
            result,
            Value::test_list(vec![
                Value::test_list(vec![Value::test_string("a"), Value::test_string("b")]),
                Value::test_list(vec![Value::test_string("a"), Value::test_string("c")]),
                Value::test_list(vec![Value::test_string("b"), Value::test_string("c")]),
            ])
        );
        Ok(())
    }

    #[test]
    fn combinations_k0_empty_input() -> Result {
        // C(0,0) = 1: the empty combination is still valid.
        let result: Value = test().run("[] | combinations 0")?;
        assert_eq!(result, Value::test_list(vec![Value::test_list(vec![])]));
        Ok(())
    }
}
