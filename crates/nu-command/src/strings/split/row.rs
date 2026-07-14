use fancy_regex::{Regex, escape};
use nu_engine::command_prelude::*;
use nu_protocol::shell_error::generic::GenericError;

use super::split;

#[derive(Clone)]
pub struct SplitRow;

impl Command for SplitRow {
    fn name(&self) -> &str {
        "split row"
    }

    fn signature(&self) -> Signature {
        Signature::build("split row")
            .input_output_types(vec![
                (Type::String, Type::List(Box::new(Type::String))),
                (
                    Type::List(Box::new(Type::String)),
                    (Type::List(Box::new(Type::String))),
                ),
            ])
            .allow_variants_without_examples(true)
            .required(
                "separator",
                SyntaxShape::String,
                "A character or regex that denotes what separates rows.",
            )
            .named(
                "number",
                SyntaxShape::Int,
                "Split into maximum number of items.",
                Some('n'),
            )
            .switch(
                "right",
                "When `--number` is used, collect the remainder in the leftmost item.",
                None,
            )
            .switch("regex", "Use regex syntax for separator.", Some('r'))
            .category(Category::Strings)
    }

    fn description(&self) -> &str {
        "Split a string into multiple rows using a separator."
    }

    fn search_terms(&self) -> Vec<&str> {
        vec!["separate", "divide", "regex"]
    }

    fn examples(&self) -> Vec<Example<'_>> {
        vec![
            Example {
                description: "Split a string into rows of char.",
                example: "'abc' | split row ''",
                result: Some(Value::list(
                    vec![
                        Value::test_string(""),
                        Value::test_string("a"),
                        Value::test_string("b"),
                        Value::test_string("c"),
                        Value::test_string(""),
                    ],
                    Span::test_data(),
                )),
            },
            Example {
                description: "Split a string into rows by the specified separator.",
                example: "'a--b--c' | split row '--'",
                result: Some(Value::list(
                    vec![
                        Value::test_string("a"),
                        Value::test_string("b"),
                        Value::test_string("c"),
                    ],
                    Span::test_data(),
                )),
            },
            Example {
                description: "Split a string by '-'.",
                example: "'-a-b-c-' | split row '-'",
                result: Some(Value::list(
                    vec![
                        Value::test_string(""),
                        Value::test_string("a"),
                        Value::test_string("b"),
                        Value::test_string("c"),
                        Value::test_string(""),
                    ],
                    Span::test_data(),
                )),
            },
            Example {
                description: "Split a string by regex.",
                example: r"'a   b       c' | split row -r '\s+'",
                result: Some(Value::list(
                    vec![
                        Value::test_string("a"),
                        Value::test_string("b"),
                        Value::test_string("c"),
                    ],
                    Span::test_data(),
                )),
            },
            Example {
                description: "Split a string a limited number of times.",
                example: "'a-b-c' | split row --number 2 '-'",
                result: Some(Value::list(
                    vec![Value::test_string("a"), Value::test_string("b-c")],
                    Span::test_data(),
                )),
            },
            Example {
                description: "Split a string a limited number of times, starting from the right.",
                example: "'a-b-c' | split row --number 2 --right '-'",
                result: Some(Value::list(
                    vec![Value::test_string("a-b"), Value::test_string("c")],
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
        let separator: Spanned<String> = call.req(engine_state, stack, 0)?;
        let max_split: Option<usize> = call.get_flag(engine_state, stack, "number")?;
        let split_from_right = call.has_flag(engine_state, stack, "right")?;
        let has_regex = call.has_flag(engine_state, stack, "regex")?;

        let args = Arguments {
            separator,
            max_split,
            split_from_right,
            has_regex,
        };
        split_row(engine_state, call, input, args)
    }

    fn run_const(
        &self,
        working_set: &StateWorkingSet,
        call: &Call,
        input: PipelineData,
    ) -> Result<PipelineData, ShellError> {
        let separator: Spanned<String> = call.req_const(working_set, 0)?;
        let max_split: Option<usize> = call.get_flag_const(working_set, "number")?;
        let split_from_right = call.has_flag_const(working_set, "right")?;
        let has_regex = call.has_flag_const(working_set, "regex")?;

        let args = Arguments {
            separator,
            max_split,
            split_from_right,
            has_regex,
        };
        split_row(working_set.permanent(), call, input, args)
    }
}

struct Arguments {
    has_regex: bool,
    separator: Spanned<String>,
    max_split: Option<usize>,
    split_from_right: bool,
}

fn split_row(
    engine_state: &EngineState,
    call: &Call,
    input: PipelineData,
    args: Arguments,
) -> Result<PipelineData, ShellError> {
    let name_span = call.head;
    let regex = if args.has_regex {
        Regex::new(&args.separator.item)
    } else {
        let escaped = escape(&args.separator.item);
        Regex::new(&escaped)
    }
    .map_err(|e| {
        ShellError::Generic(GenericError::new(
            "Error with regular expression",
            e.to_string(),
            args.separator.span,
        ))
    })?;
    input.flat_map(
        move |x| split_row_helper(&x, &regex, args.max_split, args.split_from_right, name_span),
        engine_state.signals(),
    )
}

fn split_row_helper(
    v: &Value,
    regex: &Regex,
    max_split: Option<usize>,
    split_from_right: bool,
    name: Span,
) -> Vec<Value> {
    let span = v.span();
    if let Value::Error { error, .. } = v {
        return vec![Value::error(*error.clone(), span)];
    }
    let Ok(s) = v.coerce_str() else {
        return vec![Value::error(
            ShellError::OnlySupportsThisInputType {
                exp_input_type: "string".into(),
                wrong_type: v.get_type().to_string(),
                dst_span: name,
                src_span: span,
            },
            name,
        )];
    };

    match (max_split, split_from_right) {
        (Some(0), _) => Ok(vec![]),
        (Some(max_split), true) => regex
            .find_iter(&s)
            .map(|x| x.map(|x| (x.start(), x.end())))
            .collect::<Result<Vec<_>, _>>()
            .map(|sep_bounds| {
                split(&s, sep_bounds.into_iter().rev().take(max_split - 1).rev())
                    .map(|val| Value::string(val, span))
                    .collect()
            }),
        (Some(max_split), false) => regex
            .splitn(&s, max_split)
            .map(|x| x.map(|val| Value::string(val, span)))
            .collect(),
        (None, _) => regex
            .split(&s)
            .map(|x| x.map(|val| Value::string(val, span)))
            .collect(),
    }
    .unwrap_or_else(|err| {
        vec![Value::error(
            ShellError::Generic(GenericError::new(
                "Error executing regular expression",
                err.to_string(),
                span,
            )),
            span,
        )]
    })
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test_examples() -> nu_test_support::Result {
        nu_test_support::test().examples(SplitRow)
    }
}
