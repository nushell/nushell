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
    find: String,
    replace: String,
    column_paths: Vec<CellPath>,
    literal_replace: bool,
    no_regex: bool,
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

    let options = Arc::new(Arguments {
        all: call.has_flag("all"),
        find: find.item,
        replace: replace.item,
        column_paths: call.rest(engine_state, stack, 2)?,
        literal_replace,
        no_regex,
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
            let FindReplace(find, replacement) = FindReplace(find, replace);
            if *no_regex {
                // just use regular string replacement vs regular expressions
                if *all {
                    Value::String {
                        val: val.replace(find, replacement),
                        span: head,
                    }
                } else {
                    Value::String {
                        val: val.replacen(find, replacement, 1),
                        span: head,
                    }
                }
            } else {
                // use regular expressions to replace strings
                let regex = Regex::new(find);

                match regex {
                    Ok(re) => {
                        if *all {
                            Value::String {
                                val: {
                                    if *literal_replace {
                                        re.replace_all(val, NoExpand(replacement)).to_string()
                                    } else {
                                        re.replace_all(val, replacement).to_string()
                                    }
                                },
                                span: head,
                            }
                        } else {
                            Value::String {
                                val: {
                                    if *literal_replace {
                                        re.replace(val, NoExpand(replacement)).to_string()
                                    } else {
                                        re.replace(val, replacement).to_string()
                                    }
                                },
                                span: head,
                            }
                        }
                    }
                    Err(_) => Value::String {
                        val: val.to_string(),
                        span: head,
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
            find: String::from("Cargo.(.+)"),
            replace: String::from("Carga.$1"),
            column_paths: vec![],
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
