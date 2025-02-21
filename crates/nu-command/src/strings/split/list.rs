use fancy_regex::Regex;
use nu_engine::command_prelude::*;
use nu_protocol::Signals;

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
        split_list(engine_state, call, input, has_regex, separator)
    }

    fn run_const(
        &self,
        working_set: &StateWorkingSet,
        call: &Call,
        input: PipelineData,
    ) -> Result<PipelineData, ShellError> {
        let has_regex = call.has_flag_const(working_set, "regex")?;
        let separator: Value = call.req_const(working_set, 0)?;
        split_list(working_set.permanent(), call, input, has_regex, separator)
    }
}

enum Matcher {
    Regex(Regex),
    Direct(Value),
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

    pub fn compare(&self, rhs: &Value) -> Result<bool, ShellError> {
        Ok(match self {
            Matcher::Regex(regex) => {
                if let Ok(rhs_str) = rhs.coerce_str() {
                    regex.is_match(&rhs_str).unwrap_or(false)
                } else {
                    false
                }
            }
            Matcher::Direct(lhs) => rhs == lhs,
        })
    }
}

fn split_list(
    engine_state: &EngineState,
    call: &Call,
    input: PipelineData,
    has_regex: bool,
    separator: Value,
) -> Result<PipelineData, ShellError> {
    let matcher = Matcher::new(has_regex, separator)?;
    let head = call.head;
    Ok(SplitList::new(
        input.into_iter(),
        engine_state.signals().clone(),
        move |x| matcher.compare(x).unwrap_or(false),
    )
    .map(move |x| Value::list(x, head))
    .into_pipeline_data(head, engine_state.signals().clone()))
}

struct SplitList<I, F> {
    iterator: I,
    closure: F,
    done: bool,
    signals: Signals,
}

impl<I, F> SplitList<I, F>
where
    I: Iterator,
    F: FnMut(&I::Item) -> bool,
{
    fn new(iterator: I, signals: Signals, closure: F) -> Self {
        Self {
            iterator,
            closure,
            done: false,
            signals,
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

impl<I, F> Iterator for SplitList<I, F>
where
    I: Iterator,
    F: FnMut(&I::Item) -> bool,
{
    type Item = Vec<I::Item>;

    fn next(&mut self) -> Option<Self::Item> {
        if self.done {
            return None;
        }

        let mut items = vec![];

        loop {
            match self.inner_iterator_next() {
                None => {
                    self.done = true;
                    return Some(items);
                }
                Some(value) => {
                    if (self.closure)(&value) {
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
