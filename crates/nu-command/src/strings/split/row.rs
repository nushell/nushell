use nu_engine::command_prelude::*;

use regex::Regex;

#[derive(Clone)]
pub struct SubCommand;

impl Command for SubCommand {
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
                "Split into maximum number of items",
                Some('n'),
            )
            .switch("regex", "use regex syntax for separator", Some('r'))
            .category(Category::Strings)
    }

    fn usage(&self) -> &str {
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

        let args = Arguments {
            separator,
            max_split,
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
        let has_regex = call.has_flag_const(working_set, "regex")?;

        let args = Arguments {
            separator,
            max_split,
            has_regex,
        };
        split_row(working_set.permanent(), call, input, args)
    }
}

struct Arguments {
    has_regex: bool,
    separator: Spanned<String>,
    max_split: Option<usize>,
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
        let escaped = regex::escape(&args.separator.item);
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
        move |x| split_row_helper(&x, &regex, args.max_split, name_span),
        engine_state.ctrlc.clone(),
    )
}

fn split_row_helper(v: &Value, regex: &Regex, max_split: Option<usize>, name: Span) -> Vec<Value> {
    let span = v.span();
    match v {
        Value::Error { error, .. } => {
            vec![Value::error(*error.clone(), span)]
        }
        v => {
            let v_span = v.span();

            if let Ok(s) = v.coerce_str() {
                match max_split {
                    Some(max_split) => regex
                        .splitn(&s, max_split)
                        .map(|x: &str| Value::string(x, v_span))
                        .collect(),
                    None => regex
                        .split(&s)
                        .map(|x: &str| Value::string(x, v_span))
                        .collect(),
                }
            } else {
                vec![Value::error(
                    ShellError::PipelineMismatch {
                        exp_input_type: "string".into(),
                        dst_span: name,
                        src_span: v_span,
                    },
                    name,
                )]
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
