use super::helpers::{SplitWhere, split_str};
use fancy_regex::{Regex, escape};
use nu_engine::command_prelude::*;

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
            .switch("collapse-empty", "remove empty columns", Some('c'))
            .named(
                "number",
                SyntaxShape::Int,
                "SplitWhere into maximum number of items",
                Some('n'),
            )
            .switch("regex", "use regex syntax for separator", Some('r'))
            .named(
                "split",
                SyntaxShape::String,
                "Whether to split lists before, after, or on (default) the separator",
                None,
            )
            .category(Category::Strings)
    }

    fn description(&self) -> &str {
        "Split a string into multiple rows using a separator."
    }

    fn search_terms(&self) -> Vec<&str> {
        vec!["separate", "divide", "regex"]
    }

    fn examples(&self) -> Vec<Example> {
        vec![
            Example {
                description: "Split a string into rows of char",
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
                description: "Split a string into rows by the specified separator",
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
                description: "Split a string by '-'",
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
                description: "Split a string by regex",
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
                description: "Split into rows, keeping the delimiter as part of the output",
                example: r#""7 oranges 3 bananas 5 green apples" | split row -r '\d' --split before --collapse-empty"#,
                result: Some(Value::test_list(vec![
                    Value::test_string("7 oranges "),
                    Value::test_string("3 bananas "),
                    Value::test_string("5 green apples"),
                ])),
            },
            Example {
                description: "Split into 5 rows, collapsing the empty rows and not counting them",
                example: r#""first|second||fourth|fifth|sixth|seventh" | split row "|" --number 5 --collapse-empty"#,
                result: Some(Value::test_list(vec![
                    Value::test_string("first"),
                    Value::test_string("second"),
                    Value::test_string("fourth"),
                    Value::test_string("fifth"),
                    Value::test_string("sixth|seventh"),
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
        let separator: Spanned<String> = call.req(engine_state, stack, 0)?;
        let max_split: Option<usize> = call.get_flag(engine_state, stack, "number")?;
        let has_regex = call.has_flag(engine_state, stack, "regex")?;
        let collapse_empty = call.has_flag(engine_state, stack, "collapse-empty")?;
        let split: Option<SplitWhere> = call.get_flag(engine_state, stack, "split")?;
        let split = split.unwrap_or(SplitWhere::On);

        let args = Arguments {
            separator,
            max_split,
            has_regex,
            collapse_empty,
            split,
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
        let has_regex = call.has_flag_const(working_set, "regex")?;
        let collapse_empty = call.has_flag_const(working_set, "collapse-empty")?;
        let split: Option<SplitWhere> = call.get_flag_const(working_set, "split")?;
        let split = split.unwrap_or(SplitWhere::On);

        let args = Arguments {
            separator,
            max_split,
            has_regex,
            collapse_empty,
            split,
        };
        split_row(working_set.permanent(), call, input, args)
    }
}

struct Arguments {
    has_regex: bool,
    separator: Spanned<String>,
    max_split: Option<usize>,
    collapse_empty: bool,
    split: SplitWhere,
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
    .map_err(|e| ShellError::GenericError {
        error: "Error with regular expression".into(),
        msg: e.to_string(),
        span: Some(args.separator.span),
        help: None,
        inner: vec![],
    })?;
    input.flat_map(
        move |x| match split_row_helper(
            &x,
            &regex,
            args.max_split,
            args.collapse_empty,
            args.split,
            name_span,
        ) {
            Ok(v) => v,
            Err(err) => vec![Value::error(err, x.span())],
        },
        engine_state.signals(),
    )
}

fn split_row_helper(
    v: &Value,
    regex: &Regex,
    max_split: Option<usize>,
    collapse_empty: bool,
    split: SplitWhere,
    name: Span,
) -> Result<Vec<Value>, ShellError> {
    match v {
        Value::Error { error, .. } => Err(*error.clone()),
        v => {
            let v_span = v.span();

            if let Ok(s) = v.coerce_str() {
                split_str(&s, regex, max_split, collapse_empty, split, v_span)
            } else {
                Err(ShellError::OnlySupportsThisInputType {
                    exp_input_type: "string".into(),
                    wrong_type: v.get_type().to_string(),
                    dst_span: name,
                    src_span: v_span,
                })
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

        test_examples(SplitRow {})
    }
}
