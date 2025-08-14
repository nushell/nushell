use fancy_regex::Regex;
use nu_engine::{ClosureEval, command_prelude::*};
use nu_protocol::{FromValue, Signals};

#[derive(Clone)]
pub struct SubCommand;

impl Command for SubCommand {
    fn name(&self) -> &str {
        "split list"
    }

    fn signature(&self) -> Signature {
        Signature::build("split list")
            .input_output_types(vec![(
                Type::List(Box::new(Type::Any)),
                Type::List(Box::new(Type::List(Box::new(Type::Any)))),
            )])
            .required(
                "separator",
                SyntaxShape::Any,
                "The value that denotes what separates the list.",
            )
            .switch(
                "regex",
                "separator is a regular expression, matching values that can be coerced into a string",
                Some('r'))
            .named("split", SyntaxShape::String, "Whether to split lists before, after, or on (default) the separator", None)
            .category(Category::Filters)
    }

    fn description(&self) -> &str {
        "Split a list into multiple lists using a separator."
    }

    fn search_terms(&self) -> Vec<&str> {
        vec!["separate", "divide", "regex"]
    }

    fn examples(&self) -> Vec<Example> {
        vec![
            Example {
                description: "Split a list of chars into two lists",
                example: "[a, b, c, d, e, f, g] | split list d",
                result: Some(Value::list(
                    vec![
                        Value::list(
                            vec![
                                Value::test_string("a"),
                                Value::test_string("b"),
                                Value::test_string("c"),
                            ],
                            Span::test_data(),
                        ),
                        Value::list(
                            vec![
                                Value::test_string("e"),
                                Value::test_string("f"),
                                Value::test_string("g"),
                            ],
                            Span::test_data(),
                        ),
                    ],
                    Span::test_data(),
                )),
            },
            Example {
                description: "Split a list of lists into two lists of lists",
                example: "[[1,2], [2,3], [3,4]] | split list [2,3]",
                result: Some(Value::list(
                    vec![
                        Value::list(
                            vec![Value::list(
                                vec![Value::test_int(1), Value::test_int(2)],
                                Span::test_data(),
                            )],
                            Span::test_data(),
                        ),
                        Value::list(
                            vec![Value::list(
                                vec![Value::test_int(3), Value::test_int(4)],
                                Span::test_data(),
                            )],
                            Span::test_data(),
                        ),
                    ],
                    Span::test_data(),
                )),
            },
            Example {
                description: "Split a list of chars into two lists",
                example: "[a, b, c, d, a, e, f, g] | split list a",
                result: Some(Value::list(
                    vec![
                        Value::list(vec![], Span::test_data()),
                        Value::list(
                            vec![
                                Value::test_string("b"),
                                Value::test_string("c"),
                                Value::test_string("d"),
                            ],
                            Span::test_data(),
                        ),
                        Value::list(
                            vec![
                                Value::test_string("e"),
                                Value::test_string("f"),
                                Value::test_string("g"),
                            ],
                            Span::test_data(),
                        ),
                    ],
                    Span::test_data(),
                )),
            },
            Example {
                description: "Split a list of chars into lists based on multiple characters",
                example: r"[a, b, c, d, a, e, f, g] | split list --regex '(b|e)'",
                result: Some(Value::list(
                    vec![
                        Value::list(vec![Value::test_string("a")], Span::test_data()),
                        Value::list(
                            vec![
                                Value::test_string("c"),
                                Value::test_string("d"),
                                Value::test_string("a"),
                            ],
                            Span::test_data(),
                        ),
                        Value::list(
                            vec![Value::test_string("f"), Value::test_string("g")],
                            Span::test_data(),
                        ),
                    ],
                    Span::test_data(),
                )),
            },
            Example {
                description: "Split a list of numbers on multiples of 3",
                example: r"[1 2 3 4 5 6 7 8 9 10] | split list {|e| $e mod 3 == 0 }",
                result: Some(Value::test_list(vec![
                    Value::test_list(vec![Value::test_int(1), Value::test_int(2)]),
                    Value::test_list(vec![Value::test_int(4), Value::test_int(5)]),
                    Value::test_list(vec![Value::test_int(7), Value::test_int(8)]),
                    Value::test_list(vec![Value::test_int(10)]),
                ])),
            },
            Example {
                description: "Split a list of numbers into lists ending with 0",
                example: r"[1 2 0 3 4 5 0 6 0 0 7] | split list --split after 0",
                result: Some(Value::test_list(vec![
                    Value::test_list(vec![
                        Value::test_int(1),
                        Value::test_int(2),
                        Value::test_int(0),
                    ]),
                    Value::test_list(vec![
                        Value::test_int(3),
                        Value::test_int(4),
                        Value::test_int(5),
                        Value::test_int(0),
                    ]),
                    Value::test_list(vec![Value::test_int(6), Value::test_int(0)]),
                    Value::test_list(vec![Value::test_int(0)]),
                    Value::test_list(vec![Value::test_int(7)]),
                ])),
            },
        ]
    }

    fn is_const(&self) -> bool {
        true
    }

    fn run(
        &self,
        engine_state: &EngineState,
        stack: &mut Stack,
        call: &Call,
        input: PipelineData,
    ) -> Result<PipelineData, ShellError> {
        let has_regex = call.has_flag(engine_state, stack, "regex")?;
        let separator: Value = call.req(engine_state, stack, 0)?;
        let split: Option<Split> = call.get_flag(engine_state, stack, "split")?;
        let split = split.unwrap_or(Split::On);
        let matcher = match separator {
            Value::Closure { val, .. } => {
                Matcher::from_closure(ClosureEval::new(engine_state, stack, *val))
            }
            _ => Matcher::new(has_regex, separator)?,
        };
        split_list(engine_state, call, input, matcher, split)
    }

    fn run_const(
        &self,
        working_set: &StateWorkingSet,
        call: &Call,
        input: PipelineData,
    ) -> Result<PipelineData, ShellError> {
        let has_regex = call.has_flag_const(working_set, "regex")?;
        let separator: Value = call.req_const(working_set, 0)?;
        let split: Option<Split> = call.get_flag_const(working_set, "split")?;
        let split = split.unwrap_or(Split::On);
        let matcher = Matcher::new(has_regex, separator)?;
        split_list(working_set.permanent(), call, input, matcher, split)
    }
}

enum Matcher {
    Regex(Regex),
    Direct(Value),
    Closure(Box<ClosureEval>),
}

enum Split {
    On,
    Before,
    After,
}

impl FromValue for Split {
    fn from_value(v: Value) -> Result<Self, ShellError> {
        let span = v.span();
        let s = <String>::from_value(v)?;
        match s.as_str() {
            "on" => Ok(Split::On),
            "before" => Ok(Split::Before),
            "after" => Ok(Split::After),
            _ => Err(ShellError::InvalidValue {
                valid: "one of: on, before, after".into(),
                actual: s,
                span,
            }),
        }
    }
}

impl Matcher {
    pub fn new(regex: bool, lhs: Value) -> Result<Self, ShellError> {
        if regex {
            Ok(Matcher::Regex(Regex::new(&lhs.coerce_str()?).map_err(
                |e| ShellError::GenericError {
                    error: "Error with regular expression".into(),
                    msg: e.to_string(),
                    span: match lhs {
                        Value::Error { .. } => None,
                        _ => Some(lhs.span()),
                    },
                    help: None,
                    inner: vec![],
                },
            )?))
        } else {
            Ok(Matcher::Direct(lhs))
        }
    }

    pub fn from_closure(closure: ClosureEval) -> Self {
        Self::Closure(Box::new(closure))
    }

    pub fn compare(&mut self, rhs: &Value) -> Result<bool, ShellError> {
        Ok(match self {
            Matcher::Regex(regex) => {
                if let Ok(rhs_str) = rhs.coerce_str() {
                    regex.is_match(&rhs_str).unwrap_or(false)
                } else {
                    false
                }
            }
            Matcher::Direct(lhs) => rhs == lhs,
            Matcher::Closure(closure) => closure
                .run_with_value(rhs.clone())
                .and_then(|data| data.into_value(Span::unknown()))
                .map(|value| value.is_true())
                .unwrap_or(false),
        })
    }
}

fn split_list(
    engine_state: &EngineState,
    call: &Call,
    input: PipelineData,
    mut matcher: Matcher,
    split: Split,
) -> Result<PipelineData, ShellError> {
    let head = call.head;
    Ok(SplitList::new(
        input.into_iter(),
        engine_state.signals().clone(),
        split,
        move |x| matcher.compare(x).unwrap_or(false),
    )
    .map(move |x| Value::list(x, head))
    .into_pipeline_data(head, engine_state.signals().clone()))
}

struct SplitList<I, T, F> {
    iterator: I,
    closure: F,
    done: bool,
    signals: Signals,
    split: Split,
    last_item: Option<T>,
}

impl<I, T, F> SplitList<I, T, F>
where
    I: Iterator<Item = T>,
    F: FnMut(&I::Item) -> bool,
{
    fn new(iterator: I, signals: Signals, split: Split, closure: F) -> Self {
        Self {
            iterator,
            closure,
            done: false,
            signals,
            split,
            last_item: None,
        }
    }

    fn inner_iterator_next(&mut self) -> Option<I::Item> {
        if self.signals.interrupted() {
            self.done = true;
            return None;
        }
        self.iterator.next()
    }
}

impl<I, T, F> Iterator for SplitList<I, T, F>
where
    I: Iterator<Item = T>,
    F: FnMut(&I::Item) -> bool,
{
    type Item = Vec<I::Item>;

    fn next(&mut self) -> Option<Self::Item> {
        if self.done {
            return None;
        }

        let mut items = vec![];
        if let Some(item) = self.last_item.take() {
            items.push(item);
        }

        loop {
            match self.inner_iterator_next() {
                None => {
                    self.done = true;
                    return Some(items);
                }
                Some(value) => {
                    if (self.closure)(&value) {
                        match self.split {
                            Split::On => {}
                            Split::Before => {
                                self.last_item = Some(value);
                            }
                            Split::After => {
                                items.push(value);
                            }
                        }
                        return Some(items);
                    } else {
                        items.push(value);
                    }
                }
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

        test_examples(SubCommand {})
    }
}
