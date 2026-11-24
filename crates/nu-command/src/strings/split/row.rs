use fancy_regex::{Regex, escape};
use nu_engine::command_prelude::*;
use nu_protocol::{ListStream, Signals, shell_error::generic::GenericError};

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

    fn extra_description(&self) -> &str {
        "Byte streams can yield rows before the input ends. With --number, --right, or --regex,\n\
         or if the separator is empty or ends in a carriage return or newline,\n\
         split row reads the full input first."
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
        stack: &mut Stack,
        call: &Call,
        input: PipelineData,
    ) -> Result<PipelineData, ShellError> {
        let separator: Spanned<String> = call.req_const(working_set, stack, 0)?;
        let max_split: Option<usize> = call.get_flag_const(working_set, stack, "number")?;
        let split_from_right = call.has_flag_const(working_set, stack, "right")?;
        let has_regex = call.has_flag_const(working_set, stack, "regex")?;

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
    let separator = args.separator.item.as_str();
    // A separator ending in CR or LF may itself be removed by the EOF trim.
    if !args.has_regex
        && args.max_split.is_none()
        && !args.split_from_right
        && !separator.is_empty()
        && !separator.ends_with(['\r', '\n'])
        && let PipelineData::ByteStream(stream, metadata) = input
    {
        let span = stream.span();
        let caller_spans = stream.get_caller_spans().clone();
        let signals = engine_state.signals().clone();
        let mut split = Some(stream.split_with_exit_status(separator.as_bytes().to_vec())?);
        let rows = std::iter::from_fn(move || {
            let parts = split.as_mut()?;
            if parts.is_finished() {
                return None;
            }
            let row = match signals
                .check(&span)
                .and_then(|()| parts.next().transpose())
                .transpose()?
            {
                Ok(bytes) => match String::from_utf8(bytes) {
                    Ok(mut row) => {
                        if parts.is_finished() {
                            row.truncate(row.trim_end_matches(['\r', '\n']).len());
                        }
                        return Some(Value::string(row, span));
                    }
                    Err(_) => {
                        // The eager path checked for upstream errors before decoding. Read the
                        // rest without splitting or retaining it so those errors still win.
                        return Some(match split.take()?.drain() {
                            Ok(()) => Value::error(
                                unsupported_input("binary".into(), name_span, span),
                                name_span,
                            ),
                            Err(err) => Value::error(err, span),
                        });
                    }
                },
                Err(err) => Value::error(err, span),
            };
            // Release the reader after a terminal error, even if the consumer keeps the iterator.
            split = None;
            Some(row)
        });
        // The iterator reports interruption as an error. A ListStream signal wrapper would
        // stop it before it could return that error.
        let mut output = ListStream::new(rows, span, Signals::empty()).with_eager_input_errors();
        for caller_span in caller_spans {
            output.push_caller_span(caller_span);
        }
        return Ok(PipelineData::list_stream(output, metadata));
    }

    let pattern = if args.has_regex {
        std::borrow::Cow::Borrowed(args.separator.item.as_str())
    } else {
        escape(&args.separator.item)
    };
    let regex = engine_state.compile_regex(&pattern, args.separator.span)?;
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
    let Ok(s) = v.coerce_string() else {
        return vec![Value::error(
            unsupported_input(v.get_type().to_string(), name, span),
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

fn unsupported_input(wrong_type: String, dst_span: Span, src_span: Span) -> ShellError {
    ShellError::OnlySupportsThisInputType {
        exp_input_type: "string".into(),
        wrong_type,
        dst_span,
        src_span,
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test_examples() -> nu_test_support::Result {
        nu_test_support::test().examples(SplitRow)
    }
}
