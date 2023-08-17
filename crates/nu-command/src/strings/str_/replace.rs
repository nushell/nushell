use fancy_regex::{NoExpand, Regex};
use nu_cmd_base::input_handler::{operate, CmdArgument};
use nu_engine::CallExt;
use nu_protocol::{
    ast::{Call, CellPath},
    engine::{Command, EngineState, Stack},
    report_error_new, Category, Example, PipelineData, ShellError, Signature, Span, Spanned,
    SyntaxShape, Type, Value,
};

struct Arguments {
    all: bool,
    find: Spanned<String>,
    replace: Spanned<String>,
    cell_paths: Option<Vec<CellPath>>,
    literal_replace: bool,
    no_regex: bool,
    multiline: bool,
}

impl CmdArgument for Arguments {
    fn take_cell_paths(&mut self) -> Option<Vec<CellPath>> {
        self.cell_paths.take()
    }
}

#[derive(Clone)]
pub struct SubCommand;

impl Command for SubCommand {
    fn name(&self) -> &str {
        "str replace"
    }

    fn signature(&self) -> Signature {
        Signature::build("str replace")
            .input_output_types(vec![
                (Type::String, Type::String),
                // TODO: clarify behavior with cellpath-rest argument
                (Type::Table(vec![]), Type::Table(vec![])),
                (Type::Record(vec![]), Type::Record(vec![])),
                (
                    Type::List(Box::new(Type::String)),
                    Type::List(Box::new(Type::String)),
                ),
            ])
            .required("find", SyntaxShape::String, "the pattern to find")
            .required("replace", SyntaxShape::String, "the replacement string")
            .rest(
                "rest",
                SyntaxShape::CellPath,
                "For a data structure input, operate on strings at the given cell paths",
            )
            .switch("all", "replace all occurrences of the pattern", Some('a'))
            .switch(
                "no-expand",
                "do not expand capture groups (like $name) in the replacement string",
                Some('n'),
            )
            .switch(
                "string",
                "DEPRECATED option, will be removed in 0.85. Substring matching is now the default.",
                Some('s'),
            )
            .switch(
                "regex",
                "match the pattern as a regular expression in the input, instead of a substring",
                Some('r'),
            )
            .switch(
                "multiline",
                "multi-line regex mode (implies --regex): ^ and $ match begin/end of line; equivalent to (?m)",
                Some('m'),
            )
            .allow_variants_without_examples(true)
            .category(Category::Strings)
    }

    fn usage(&self) -> &str {
        "Find and replace text."
    }

    fn search_terms(&self) -> Vec<&str> {
        vec!["search", "shift", "switch", "regex"]
    }

    fn run(
        &self,
        engine_state: &EngineState,
        stack: &mut Stack,
        call: &Call,
        input: PipelineData,
    ) -> Result<PipelineData, ShellError> {
        let find: Spanned<String> = call.req(engine_state, stack, 0)?;
        let replace: Spanned<String> = call.req(engine_state, stack, 1)?;
        let cell_paths: Vec<CellPath> = call.rest(engine_state, stack, 2)?;
        let cell_paths = (!cell_paths.is_empty()).then_some(cell_paths);
        let literal_replace = call.has_flag("no-expand");
        if call.has_flag("string") {
            report_error_new(
                engine_state,
                &ShellError::GenericError(
                    "Deprecated option".into(),
                    "`str replace --string` is deprecated and will be removed in 0.85.".into(),
                    Some(call.head),
                    Some("Substring matching is now the default. Use `--regex` or `--multiline` for matching regular expressions.".into()),
                    vec![],
                ),
            );
        }
        let no_regex = !call.has_flag("regex") && !call.has_flag("multiline");
        let multiline = call.has_flag("multiline");

        let args = Arguments {
            all: call.has_flag("all"),
            find,
            replace,
            cell_paths,
            literal_replace,
            no_regex,
            multiline,
        };
        operate(action, args, input, call.head, engine_state.ctrlc.clone())
    }

    fn examples(&self) -> Vec<Example> {
        vec![
            Example {
                description: "Find and replace the first occurrence of a substring",
                example: r"'c:\some\cool\path' | str replace 'c:\some\cool' '~'",
                result: Some(Value::test_string("~\\path")),
            },
            Example {
                description: "Find and replace all occurrences of a substring",
                example: r#"'abc abc abc' | str replace -a 'b' 'z'"#,
                result: Some(Value::test_string("azc azc azc")),
            },
            Example {
                description: "Find and replace contents with capture group using regular expression",
                example: "'my_library.rb' | str replace -r '(.+).rb' '$1.nu'",
                result: Some(Value::test_string("my_library.nu")),
            },
            Example {
                description: "Find and replace all occurrences of find string using regular expression",
                example: "'abc abc abc' | str replace -ar 'b' 'z'",
                result: Some(Value::test_string("azc azc azc")),
            },
            Example {
                description: "Find and replace all occurrences of find string in table using regular expression",
                example:
                    "[[ColA ColB ColC]; [abc abc ads]] | str replace -ar 'b' 'z' ColA ColC",
                result: Some(Value::List {
                    vals: vec![Value::Record {
                        cols: vec!["ColA".to_string(), "ColB".to_string(), "ColC".to_string()],
                        vals: vec![
                            Value::test_string("azc"),
                            Value::test_string("abc"),
                            Value::test_string("ads"),
                        ],
                        span: Span::test_data(),
                    }],
                    span: Span::test_data(),
                }),
            },
            Example {
                description: "Find and replace all occurrences of find string in record using regular expression",
                example:
                    "{ KeyA: abc, KeyB: abc, KeyC: ads } | str replace -ar 'b' 'z' KeyA KeyC",
                result: Some(Value::Record {
                        cols: vec!["KeyA".to_string(), "KeyB".to_string(), "KeyC".to_string()],
                        vals: vec![
                            Value::test_string("azc"),
                            Value::test_string("abc"),
                            Value::test_string("ads"),
                        ],
                        span: Span::test_data(),
                    }),
            },
            Example {
                description: "Find and replace contents without using the replace parameter as a regular expression",
                example: r"'dogs_$1_cats' | str replace -r '\$1' '$2' -n",
                result: Some(Value::test_string("dogs_$2_cats")),
            },
            Example {
                description: "Use captures to manipulate the input text using regular expression",
                example: r#""abc-def" | str replace -r "(.+)-(.+)" "${2}_${1}""#,
                result: Some(Value::test_string("def_abc")),
            },
            Example {
                description: "Find and replace with fancy-regex using regular expression",
                example: r"'a successful b' | str replace -r '\b([sS])uc(?:cs|s?)e(ed(?:ed|ing|s?)|ss(?:es|ful(?:ly)?|i(?:ons?|ve(?:ly)?)|ors?)?)\b' '${1}ucce$2'",
                result: Some(Value::test_string("a successful b")),
            },
            Example {
                description: "Find and replace with fancy-regex using regular expression",
                example: r#"'GHIKK-9+*' | str replace -r '[*[:xdigit:]+]' 'z'"#,
                result: Some(Value::test_string("GHIKK-z+*")),
            },
            Example {
                description: "Find and replace on individual lines using multiline regular expression",
                example: r#""non-matching line\n123. one line\n124. another line\n" | str replace -am '^[0-9]+\. ' ''"#,
                result: Some(Value::test_string("non-matching line\none line\nanother line\n")),
            },

        ]
    }
}

struct FindReplace<'a>(&'a str, &'a str);

fn action(
    input: &Value,
    Arguments {
        find,
        replace,
        all,
        literal_replace,
        no_regex,
        multiline,
        ..
    }: &Arguments,
    head: Span,
) -> Value {
    match input {
        Value::String { val, .. } => {
            let FindReplace(find_str, replace_str) = FindReplace(&find.item, &replace.item);
            if *no_regex {
                // just use regular string replacement vs regular expressions
                if *all {
                    Value::String {
                        val: val.replace(find_str, replace_str),
                        span: head,
                    }
                } else {
                    Value::String {
                        val: val.replacen(find_str, replace_str, 1),
                        span: head,
                    }
                }
            } else {
                // use regular expressions to replace strings
                let flags = match multiline {
                    true => "(?m)",
                    false => "",
                };
                let regex_string = flags.to_string() + find_str;
                let regex = Regex::new(&regex_string);

                match regex {
                    Ok(re) => {
                        if *all {
                            Value::String {
                                val: {
                                    if *literal_replace {
                                        re.replace_all(val, NoExpand(replace_str)).to_string()
                                    } else {
                                        re.replace_all(val, replace_str).to_string()
                                    }
                                },
                                span: head,
                            }
                        } else {
                            Value::String {
                                val: {
                                    if *literal_replace {
                                        re.replace(val, NoExpand(replace_str)).to_string()
                                    } else {
                                        re.replace(val, replace_str).to_string()
                                    }
                                },
                                span: head,
                            }
                        }
                    }
                    Err(e) => Value::Error {
                        error: Box::new(ShellError::IncorrectValue {
                            msg: format!("Regex error: {e}"),
                            span: find.span,
                        }),
                    },
                }
            }
        }
        Value::Error { .. } => input.clone(),
        _ => Value::Error {
            error: Box::new(ShellError::OnlySupportsThisInputType {
                exp_input_type: "string".into(),
                wrong_type: input.get_type().to_string(),
                dst_span: head,
                src_span: input.expect_span(),
            }),
        },
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use super::{action, Arguments, SubCommand};

    fn test_spanned_string(val: &str) -> Spanned<String> {
        Spanned {
            item: String::from(val),
            span: Span::test_data(),
        }
    }

    #[test]
    fn test_examples() {
        use crate::test_examples;

        test_examples(SubCommand {})
    }

    #[test]
    fn can_have_capture_groups() {
        let word = Value::test_string("Cargo.toml");

        let options = Arguments {
            find: test_spanned_string("Cargo.(.+)"),
            replace: test_spanned_string("Carga.$1"),
            cell_paths: None,
            literal_replace: false,
            all: false,
            no_regex: false,
            multiline: false,
        };

        let actual = action(&word, &options, Span::test_data());
        assert_eq!(actual, Value::test_string("Carga.toml"));
    }
}
