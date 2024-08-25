use itertools::{Itertools, MultiPeek};
use nu_engine::command_prelude::*;

#[derive(Clone)]
pub struct Drop;

impl Command for Drop {
    fn name(&self) -> &str {
        "drop"
    }

    fn signature(&self) -> Signature {
        Signature::build("drop")
            .input_output_types(vec![
                (Type::table(), Type::table()),
                (
                    Type::List(Box::new(Type::Any)),
                    Type::List(Box::new(Type::Any)),
                ),
            ])
            .optional("rows", SyntaxShape::Int, "The number of items to remove.")
            .category(Category::Filters)
    }

    fn description(&self) -> &str {
        "Remove items/rows from the end of the input list/table. Counterpart of `skip`. Opposite of `last`."
    }

    fn search_terms(&self) -> Vec<&str> {
        vec!["delete"]
    }

    fn examples(&self) -> Vec<Example> {
        vec![
            Example {
                example: "[0,1,2,3] | drop",
                description: "Remove the last item of a list",
                result: Some(Value::test_list(vec![
                    Value::test_int(0),
                    Value::test_int(1),
                    Value::test_int(2),
                ])),
            },
            Example {
                example: "[0,1,2,3] | drop 0",
                description: "Remove zero item of a list",
                result: Some(Value::test_list(vec![
                    Value::test_int(0),
                    Value::test_int(1),
                    Value::test_int(2),
                    Value::test_int(3),
                ])),
            },
            Example {
                example: "[0,1,2,3] | drop 2",
                description: "Remove the last two items of a list",
                result: Some(Value::test_list(vec![
                    Value::test_int(0),
                    Value::test_int(1),
                ])),
            },
            Example {
                description: "Remove the last row in a table",
                example: "[[a, b]; [1, 2] [3, 4]] | drop 1",
                result: Some(Value::test_list(vec![Value::test_record(record! {
                    "a" => Value::test_int(1),
                    "b" => Value::test_int(2),
                })])),
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
        let head = call.head;
        let metadata = input.metadata();
        let rows: usize = call.opt(engine_state, stack, 0)?.or(Some(1)).unwrap();

        input.into_iter_strict(head).map(|iter| {
            DropIterator::new(iter, rows).into_pipeline_data_with_metadata(
                head,
                engine_state.signals().clone(),
                metadata,
            )
        })
    }
}

/// Drops the specified numbers of rows from the end of an iterator.
struct DropIterator<Item, Iter>
where
    Iter: Iterator<Item = Item>,
{
    iter: MultiPeek<Iter>,
    rows: usize,
}

impl<Item, Iter> DropIterator<Item, Iter>
where
    Iter: Iterator<Item = Item>,
{
    fn new(iter: Iter, rows: usize) -> Self {
        Self {
            iter: iter.multipeek(),
            rows,
        }
    }
}

impl<Item, Iter> Iterator for DropIterator<Item, Iter>
where
    Iter: Iterator<Item = Item>,
{
    type Item = Item;

    fn next(&mut self) -> Option<Self::Item> {
        for _ in 0..(self.rows + 1) {
            if self.iter.peek().is_none() {
                return None;
            }
        }
        self.iter.next()
    }
}

#[cfg(test)]
mod test {
    use crate::Drop;

    #[test]
    fn test_examples() {
        use crate::test_examples;

        test_examples(Drop {})
    }
}
