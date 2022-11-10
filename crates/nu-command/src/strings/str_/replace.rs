use crate::input_handler::{operate, CmdArgument};
use fancy_regex::{NoExpand, Regex};
use nu_engine::CallExt;
use nu_protocol::{
    ast::{Call, CellPath},
    engine::{Command, EngineState, Stack},
    Category, Example, PipelineData, ShellError, Signature, Span, Spanned, SyntaxShape, Type,
    Value,
};

struct Arguments {
    all: bool,
    find: Spanned<String>,
    replace: Spanned<String>,
    cell_paths: Option<Vec<CellPath>>,
    literal_replace: bool,
    no_regex: bool,
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
            .input_output_types(vec![(Type::String, Type::String)])
            .vectorizes_over_list(true)
            .required("find", SyntaxShape::String, "the pattern to find")
            .required("replace", SyntaxShape::String, "the replacement pattern")
            .rest(
                "rest",
                SyntaxShape::CellPath,
                "For a data structure input, operate on strings at the given cell paths",
            )
            .switch("all", "replace all occurrences of find string", Some('a'))
            .switch(
                "no-expand",
                "do not expand the replace parameter as a regular expression",
                Some('n'),
            )
            .switch(
                "string",
                "do not use regular expressions for string find and replace",
                Some('s'),
            )
            .category(Category::Strings)
    }

    fn usage(&self) -> &str {
        "Find and replace text"
    }

    fn search_terms(&self) -> Vec<&str> {
        vec!["search", "shift", "switch"]
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
        let no_regex = call.has_flag("string");

        let args = Arguments {
            all: call.has_flag("all"),
            find,
            replace,
            cell_paths,
            literal_replace,
            no_regex,
        };
        operate(action, args, input, call.head, engine_state.ctrlc.clone())
    }

    fn examples(&self) -> Vec<Example> {
        vec![
            Example {
                description: "Find and replace contents with capture group",
                example: "'my_library.rb' | str replace '(.+).rb' '$1.nu'",
                result: Some(Value::String {
                    val: "my_library.nu".to_string(),
                    span: Span::test_data(),
                }),
            },
            Example {
                description: "Find and replace all occurrences of find string",
                example: "'abc abc abc' | str replace -a 'b' 'z'",
                result: Some(Value::String {
                    val: "azc azc azc".to_string(),
                    span: Span::test_data(),
                }),
            },
            Example {
                description: "Find and replace all occurrences of find string in table",
                example:
                    "[[ColA ColB ColC]; [abc abc ads]] | str replace -a 'b' 'z' ColA ColC",
                result: Some(Value::List {
                    vals: vec![Value::Record {
                        cols: vec!["ColA".to_string(), "ColB".to_string(), "ColC".to_string()],
                        vals: vec![
                            Value::String {
                                val: "azc".to_string(),
                                span: Span::test_data(),
                            },
                            Value::String {
                                val: "abc".to_string(),
                                span: Span::test_data(),
                            },
                            Value::String {
                                val: "ads".to_string(),
                                span: Span::test_data(),
                            },
                        ],
                        span: Span::test_data(),
                    }],
                    span: Span::test_data(),
                }),
            },
            Example {
                description: "Find and replace contents without using the replace parameter as a regular expression",
                example: r#"'dogs_$1_cats' | str replace '\$1' '$2' -n"#,
                result: Some(Value::String {
                    val: "dogs_$2_cats".to_string(),
                    span: Span::test_data(),
                }),
            },
            Example {
                description: "Find and replace the first occurence using string replacement *not* regular expressions",
                example: r#"'c:\some\cool\path' | str replace 'c:\some\cool' '~' -s"#,
                result: Some(Value::String {
                    val: "~\\path".to_string(),
                    span: Span::test_data(),
                }),
            },
            Example {
                description: "Find and replace all occurences using string replacement *not* regular expressions",
                example: r#"'abc abc abc' | str replace -a 'b' 'z' -s"#,
                result: Some(Value::String {
                    val: "azc azc azc".to_string(),
                    span: Span::test_data(),
                }),
            },
            Example {
                description: "Find and replace with fancy-regex",
                example: r#"'a sucessful b' | str replace '\b([sS])uc(?:cs|s?)e(ed(?:ed|ing|s?)|ss(?:es|ful(?:ly)?|i(?:ons?|ve(?:ly)?)|ors?)?)\b' '${1}ucce$2'"#,
                result: Some(Value::String {
                    val: "a successful b".to_string(),
                    span: Span::test_data(),
                }),
            },
            Example {
                description: "Find and replace with fancy-regex",
                example: r#"'GHIKK-9+*' | str replace '[*[:xdigit:]+]' 'z'"#,
                result: Some(Value::String {
                    val: "GHIKK-z+*".to_string(),
                    span: Span::test_data(),
                }),
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
                let regex = Regex::new(find_str);

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
                        error: ShellError::UnsupportedInput(format!("{e}"), find.span),
                    },
                }
            }
        }
        other => Value::Error {
            error: ShellError::UnsupportedInput(
                format!(
                    "Input's type is {}. This command only works with strings.",
                    other.get_type()
                ),
                head,
            ),
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
        let word = Value::String {
            val: "Cargo.toml".to_string(),
            span: Span::test_data(),
        };

        let options = Arguments {
            find: test_spanned_string("Cargo.(.+)"),
            replace: test_spanned_string("Carga.$1"),
            cell_paths: None,
            literal_replace: false,
            all: false,
            no_regex: false,
        };

        let actual = action(&word, &options, Span::test_data());
        assert_eq!(
            actual,
            Value::String {
                val: "Carga.toml".to_string(),
                span: Span::test_data()
            }
        );
    }
}
