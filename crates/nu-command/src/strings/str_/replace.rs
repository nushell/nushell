use fancy_regex::{NoExpand as FancyNoExpand, Regex as FancyRegex};
use nu_engine::CallExt;
use nu_protocol::{
    ast::{Call, CellPath},
    engine::{Command, EngineState, Stack},
    Category, Example, PipelineData, ShellError, Signature, Span, Spanned, SyntaxShape, Value,
};
use regex::{NoExpand, Regex};
use std::sync::Arc;

struct Arguments {
    all: bool,
    find: Spanned<String>,
    replace: String,
    column_paths: Vec<CellPath>,
    literal_replace: bool,
    no_regex: bool,
    use_fancy_regex: bool,
}

#[derive(Clone)]
pub struct SubCommand;

impl Command for SubCommand {
    fn name(&self) -> &str {
        "str replace"
    }

    fn signature(&self) -> Signature {
        Signature::build("str replace")
            .required("find", SyntaxShape::String, "the pattern to find")
            .required("replace", SyntaxShape::String, "the replacement pattern")
            .rest(
                "rest",
                SyntaxShape::CellPath,
                "optionally find and replace text by column paths",
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
            .switch(
                "fancy-regex",
                "use the fancy-regex crate instead of regex crate",
                Some('f'),
            )
            .category(Category::Strings)
    }

    fn usage(&self) -> &str {
        "Find and replace text"
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
        operate(engine_state, stack, call, input)
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
                description: "Use fancy-regex crate with look-around to find and replace",
                example: r#"'AU$10, $20' | str replace -f '(?<!AU)\$(\d+)' '100'"#,
                result: Some(Value::String {
                    val: "AU$10, 100".to_string(),
                    span: Span::test_data(),
                }),
            },
            Example {
                description: "Use fancy-regex crate with atomic groups to find and replace",
                example: r#"'abcc' | str replace -f '^a(?>bc|b)c$' '-'"#,
                result: Some(Value::String {
                    val: "-".to_string(),
                    span: Span::test_data(),
                }),
            },
        ]
    }
}

fn operate(
    engine_state: &EngineState,
    stack: &mut Stack,
    call: &Call,
    input: PipelineData,
) -> Result<PipelineData, ShellError> {
    let head = call.head;
    let find: Spanned<String> = call.req(engine_state, stack, 0)?;
    let replace: Spanned<String> = call.req(engine_state, stack, 1)?;
    let literal_replace = call.has_flag("no-expand");
    let no_regex = call.has_flag("string");
    let use_fancy_regex = call.has_flag("fancy-regex");

    let options = Arc::new(Arguments {
        all: call.has_flag("all"),
        find,
        replace: replace.item,
        column_paths: call.rest(engine_state, stack, 2)?,
        literal_replace,
        no_regex,
        use_fancy_regex,
    });

    input.map(
        move |v| {
            if options.column_paths.is_empty() {
                action(&v, &options, head)
            } else {
                let mut ret = v;
                for path in &options.column_paths {
                    let opt = options.clone();
                    let r = ret.update_cell_path(
                        &path.members,
                        Box::new(move |old| action(old, &opt, head)),
                    );
                    if let Err(error) = r {
                        return Value::Error { error };
                    }
                }
                ret
            }
        },
        engine_state.ctrlc.clone(),
    )
}

struct FindReplace<'a>(&'a str, &'a str);

enum NuRegEx {
    Regex(Result<Regex, regex::Error>),
    FancyRegex(Result<FancyRegex, fancy_regex::Error>),
}

impl NuRegEx {
    fn replace(&self, val: &str, replacement: &str, literal_replace: bool, span: Span) -> Value {
        match self {
            NuRegEx::Regex(regex) => match regex {
                Ok(re) => {
                    if literal_replace {
                        Value::String {
                            val: re.replace(val, NoExpand(replacement)).to_string(),
                            span,
                        }
                    } else {
                        Value::String {
                            val: re.replace(val, replacement).to_string(),
                            span,
                        }
                    }
                }
                Err(e) => Value::Error {
                    error: ShellError::GenericError(
                        "error with regular expression".into(),
                        e.to_string(),
                        Some(span),
                        None,
                        Vec::new(),
                    ),
                },
            },
            NuRegEx::FancyRegex(regex) => match regex {
                Ok(re) => {
                    if literal_replace {
                        Value::String {
                            val: re.replace(val, FancyNoExpand(replacement)).to_string(),
                            span,
                        }
                    } else {
                        Value::String {
                            val: re.replace(val, replacement).to_string(),
                            span,
                        }
                    }
                }
                Err(e) => Value::Error {
                    error: ShellError::GenericError(
                        "error with regular expression".into(),
                        e.to_string(),
                        Some(span),
                        None,
                        Vec::new(),
                    ),
                },
            },
        }
    }
    fn replace_all(
        &self,
        val: &str,
        replacement: &str,
        literal_replace: bool,
        span: Span,
    ) -> Value {
        match self {
            NuRegEx::Regex(regex) => match regex {
                Ok(re) => {
                    if literal_replace {
                        Value::String {
                            val: re.replace_all(val, NoExpand(replacement)).to_string(),
                            span,
                        }
                    } else {
                        Value::String {
                            val: re.replace_all(val, replacement).to_string(),
                            span,
                        }
                    }
                }
                Err(e) => Value::Error {
                    error: ShellError::GenericError(
                        "error with regular expression".into(),
                        e.to_string(),
                        Some(span),
                        None,
                        Vec::new(),
                    ),
                },
            },
            NuRegEx::FancyRegex(regex) => match regex {
                Ok(re) => {
                    if literal_replace {
                        Value::String {
                            val: re.replace_all(val, FancyNoExpand(replacement)).to_string(),
                            span,
                        }
                    } else {
                        Value::String {
                            val: re.replace_all(val, replacement).to_string(),
                            span,
                        }
                    }
                }
                Err(e) => Value::Error {
                    error: ShellError::GenericError(
                        "error with regular expression".into(),
                        e.to_string(),
                        Some(span),
                        None,
                        Vec::new(),
                    ),
                },
            },
        }
    }
}

fn action(
    input: &Value,
    Arguments {
        all,
        find,
        replace,
        literal_replace,
        no_regex,
        use_fancy_regex,
        ..
    }: &Arguments,
    head: Span,
) -> Value {
    match input {
        Value::String { val, .. } => {
            let FindReplace(find_str, replacement) = FindReplace(&find.item, replace);
            if *no_regex {
                // just use regular string replacement vs regular expressions
                if *all {
                    Value::String {
                        val: val.replace(find_str, replacement),
                        span: head,
                    }
                } else {
                    Value::String {
                        val: val.replacen(find_str, replacement, 1),
                        span: head,
                    }
                }
            } else {
                // use regular expressions to replace strings
                let regex = if *use_fancy_regex {
                    NuRegEx::FancyRegex(FancyRegex::new(find_str))
                } else {
                    NuRegEx::Regex(Regex::new(find_str))
                };

                if *all {
                    regex.replace_all(val, replacement, *literal_replace, find.span)
                } else {
                    regex.replace(val, replacement, *literal_replace, find.span)
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

        let spanned_find = Spanned {
            item: "Cargo.(.+)".to_string(),
            span: Span::test_data(),
        };
        let options = Arguments {
            find: spanned_find,
            replace: String::from("Carga.$1"),
            column_paths: vec![],
            literal_replace: false,
            all: false,
            no_regex: false,
            use_fancy_regex: false,
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
