use fancy_regex::{Captures, NoExpand, Regex};
use nu_cmd_base::input_handler::{CmdArgument, operate};
use nu_engine::{ClosureEval, command_prelude::*};
use std::sync::Arc;

enum ReplacementValue {
    String(Arc<Spanned<String>>),
    Closure(Box<Spanned<ClosureEval>>),
}

struct Arguments {
    all: bool,
    find: Spanned<String>,
    replace: ReplacementValue,
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
pub struct StrReplace;

impl Command for StrReplace {
    fn name(&self) -> &str {
        "str replace"
    }

    fn signature(&self) -> Signature {
        Signature::build("str replace")
            .input_output_types(vec![
                (Type::String, Type::String),
                // TODO: clarify behavior with cell-path-rest argument
                (Type::table(), Type::table()),
                (Type::record(), Type::record()),
                (
                    Type::List(Box::new(Type::String)),
                    Type::List(Box::new(Type::String)),
                ),
            ])
            .required("find", SyntaxShape::String, "The pattern to find.")
            .required("replace",
                SyntaxShape::OneOf(vec![SyntaxShape::String, SyntaxShape::Closure(None)]),
                "The replacement string, or a closure that generates it."
            )
            .rest(
                "rest",
                SyntaxShape::CellPath,
                "For a data structure input, operate on strings at the given cell paths.",
            )
            .switch("all", "replace all occurrences of the pattern", Some('a'))
            .switch(
                "no-expand",
                "do not expand capture groups (like $name) in the replacement string",
                Some('n'),
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

    fn description(&self) -> &str {
        "Find and replace text."
    }

    fn extra_description(&self) -> &str {
        r#"The pattern to find can be a substring (default) or a regular expression (with `--regex`).

The replacement can be a a string, possibly containing references to numbered (`$1` etc) or
named capture groups (`$name`), or it can be closure that is invoked for each match.
In the latter case, the closure is invoked with the entire match as its input and any capture
groups as its argument. It must return a string that will be used as a replacement for the match.
"#
    }

    fn search_terms(&self) -> Vec<&str> {
        vec!["search", "shift", "switch", "regex"]
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
        let find: Spanned<String> = call.req(engine_state, stack, 0)?;
        let replace = match call.req(engine_state, stack, 1)? {
            Value::Closure {
                val, internal_span, ..
            } => Ok(ReplacementValue::Closure(Box::new(
                ClosureEval::new(engine_state, stack, *val).into_spanned(internal_span),
            ))),
            Value::String {
                val, internal_span, ..
            } => Ok(ReplacementValue::String(Arc::new(
                val.into_spanned(internal_span),
            ))),
            val => Err(ShellError::TypeMismatch {
                err_message: "unsupported replacement value type".to_string(),
                span: val.span(),
            }),
        }?;
        let cell_paths: Vec<CellPath> = call.rest(engine_state, stack, 2)?;
        let cell_paths = (!cell_paths.is_empty()).then_some(cell_paths);
        let literal_replace = call.has_flag(engine_state, stack, "no-expand")?;
        let no_regex = !call.has_flag(engine_state, stack, "regex")?
            && !call.has_flag(engine_state, stack, "multiline")?;
        let multiline = call.has_flag(engine_state, stack, "multiline")?;

        let args = Arguments {
            all: call.has_flag(engine_state, stack, "all")?,
            find,
            replace,
            cell_paths,
            literal_replace,
            no_regex,
            multiline,
        };
        operate(action, args, input, call.head, engine_state.signals())
    }

    fn run_const(
        &self,
        working_set: &StateWorkingSet,
        call: &Call,
        input: PipelineData,
    ) -> Result<PipelineData, ShellError> {
        let find: Spanned<String> = call.req_const(working_set, 0)?;
        let replace: Spanned<String> = call.req_const(working_set, 1)?;
        let cell_paths: Vec<CellPath> = call.rest_const(working_set, 2)?;
        let cell_paths = (!cell_paths.is_empty()).then_some(cell_paths);
        let literal_replace = call.has_flag_const(working_set, "no-expand")?;
        let no_regex = !call.has_flag_const(working_set, "regex")?
            && !call.has_flag_const(working_set, "multiline")?;
        let multiline = call.has_flag_const(working_set, "multiline")?;

        let args = Arguments {
            all: call.has_flag_const(working_set, "all")?,
            find,
            replace: ReplacementValue::String(Arc::new(replace)),
            cell_paths,
            literal_replace,
            no_regex,
            multiline,
        };
        operate(
            action,
            args,
            input,
            call.head,
            working_set.permanent().signals(),
        )
    }

    fn examples(&self) -> Vec<Example<'_>> {
        vec![
            Example {
                description: "Find and replace the first occurrence of a substring",
                example: r"'c:\some\cool\path' | str replace 'c:\some\cool' '~'",
                result: Some(Value::test_string("~\\path")),
            },
            Example {
                description: "Find and replace all occurrences of a substring",
                example: r#"'abc abc abc' | str replace --all 'b' 'z'"#,
                result: Some(Value::test_string("azc azc azc")),
            },
            Example {
                description: "Find and replace contents with capture group using regular expression",
                example: "'my_library.rb' | str replace -r '(.+).rb' '$1.nu'",
                result: Some(Value::test_string("my_library.nu")),
            },
            Example {
                description: "Find and replace contents with capture group using regular expression, with escapes",
                example: "'hello=world' | str replace -r '\\$?(?<varname>.*)=(?<value>.*)' '$$$varname = $value'",
                result: Some(Value::test_string("$hello = world")),
            },
            Example {
                description: "Find and replace all occurrences of found string using regular expression",
                example: "'abc abc abc' | str replace --all --regex 'b' 'z'",
                result: Some(Value::test_string("azc azc azc")),
            },
            Example {
                description: "Find and replace all occurrences of found string in table using regular expression",
                example: "[[ColA ColB ColC]; [abc abc ads]] | str replace --all --regex 'b' 'z' ColA ColC",
                result: Some(Value::test_list(vec![Value::test_record(record! {
                    "ColA" => Value::test_string("azc"),
                    "ColB" => Value::test_string("abc"),
                    "ColC" => Value::test_string("ads"),
                })])),
            },
            Example {
                description: "Find and replace all occurrences of found string in record using regular expression",
                example: "{ KeyA: abc, KeyB: abc, KeyC: ads } | str replace --all --regex 'b' 'z' KeyA KeyC",
                result: Some(Value::test_record(record! {
                    "KeyA" => Value::test_string("azc"),
                    "KeyB" => Value::test_string("abc"),
                    "KeyC" => Value::test_string("ads"),
                })),
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
                example: r#""non-matching line\n123. one line\n124. another line\n" | str replace --all --multiline '^[0-9]+\. ' ''"#,
                result: Some(Value::test_string(
                    "non-matching line\none line\nanother line\n",
                )),
            },
            Example {
                description: "Find and replace backslash escape sequences using a closure",
                example: r#"'string: \"abc\" backslash: \\ newline:\nend' | str replace -a -r '\\(.)' {|char| if $char == "n" { "\n" } else { $char } }"#,
                result: Some(Value::test_string(
                    "string: \"abc\" backslash: \\ newline:\nend",
                )),
            },
        ]
    }
}

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
            let find_str: &str = &find.item;
            if *no_regex {
                // just use regular string replacement vs regular expressions
                let replace_str: Result<Arc<Spanned<String>>, (ShellError, Span)> = match replace {
                    ReplacementValue::String(replace_str) => Ok(replace_str.clone()),
                    ReplacementValue::Closure(closure) => {
                        // find_str is fixed, so we need to run the closure only once
                        let mut closure_eval = closure.item.clone();
                        let span = closure.span;
                        let result: Result<Value, ShellError> = closure_eval
                            .run_with_value(Value::string(find.item.clone(), find.span))
                            .and_then(|result| result.into_value(span));
                        match result {
                            Ok(Value::String { val, .. }) => Ok(Arc::new(val.into_spanned(span))),
                            Ok(res) => Err((
                                ShellError::RuntimeTypeMismatch {
                                    expected: Type::String,
                                    actual: res.get_type(),
                                    span: res.span(),
                                },
                                span,
                            )),
                            Err(error) => Err((error, span)),
                        }
                    }
                };
                match replace_str {
                    Ok(replace_str) => {
                        if *all {
                            Value::string(val.replace(find_str, &replace_str.item), head)
                        } else {
                            Value::string(val.replacen(find_str, &replace_str.item, 1), head)
                        }
                    }
                    Err((error, span)) => Value::error(error, span),
                }
            } else {
                // use regular expressions to replace strings
                let flags = match multiline {
                    true => "(?m)",
                    false => "",
                };
                let regex_string = flags.to_string() + find_str;
                let regex = Regex::new(&regex_string);

                match (regex, replace) {
                    (Ok(re), ReplacementValue::String(replace_str)) => {
                        if *all {
                            Value::string(
                                {
                                    if *literal_replace {
                                        re.replace_all(val, NoExpand(&replace_str.item)).to_string()
                                    } else {
                                        re.replace_all(val, &replace_str.item).to_string()
                                    }
                                },
                                head,
                            )
                        } else {
                            Value::string(
                                {
                                    if *literal_replace {
                                        re.replace(val, NoExpand(&replace_str.item)).to_string()
                                    } else {
                                        re.replace(val, &replace_str.item).to_string()
                                    }
                                },
                                head,
                            )
                        }
                    }
                    (Ok(re), ReplacementValue::Closure(closure)) => {
                        let span = closure.span;
                        // TODO: We only need to clone the evaluator here because
                        //       operate() doesn't allow us to have a mutable reference
                        //       to Arguments. Would it be worth the effort to change operate()
                        //       and all commands that use it?
                        let mut closure_eval = closure.item.clone();
                        let mut first_error: Option<ShellError> = None;
                        let replacer = |caps: &Captures| {
                            for capture in caps.iter().skip(1) {
                                let arg = match capture {
                                    Some(m) => Value::string(m.as_str().to_string(), head),
                                    None => Value::nothing(head),
                                };
                                closure_eval.add_arg(arg);
                            }
                            let value = match caps.get(0) {
                                Some(m) => Value::string(m.as_str().to_string(), head),
                                None => Value::nothing(head),
                            };
                            let result: Result<Value, ShellError> = closure_eval
                                .run_with_input(PipelineData::value(value, None))
                                .and_then(|result| result.into_value(span));
                            match result {
                                Ok(Value::String { val, .. }) => val.to_string(),
                                Ok(res) => {
                                    first_error = Some(ShellError::RuntimeTypeMismatch {
                                        expected: Type::String,
                                        actual: res.get_type(),
                                        span: res.span(),
                                    });
                                    "".to_string()
                                }
                                Err(e) => {
                                    first_error = Some(e);
                                    "".to_string()
                                }
                            }
                        };
                        let result = if *all {
                            Value::string(re.replace_all(val, replacer).to_string(), head)
                        } else {
                            Value::string(re.replace(val, replacer).to_string(), head)
                        };
                        match first_error {
                            None => result,
                            Some(error) => Value::error(error, span),
                        }
                    }
                    (Err(e), _) => Value::error(
                        ShellError::IncorrectValue {
                            msg: format!("Regex error: {e}"),
                            val_span: find.span,
                            call_span: head,
                        },
                        find.span,
                    ),
                }
            }
        }
        Value::Error { .. } => input.clone(),
        _ => Value::error(
            ShellError::OnlySupportsThisInputType {
                exp_input_type: "string".into(),
                wrong_type: input.get_type().to_string(),
                dst_span: head,
                src_span: input.span(),
            },
            head,
        ),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use super::{Arguments, StrReplace, action};

    fn test_spanned_string(val: &str) -> Spanned<String> {
        Spanned {
            item: String::from(val),
            span: Span::test_data(),
        }
    }

    #[test]
    fn test_examples() {
        use crate::test_examples;

        test_examples(StrReplace {})
    }

    #[test]
    fn can_have_capture_groups() {
        let word = Value::test_string("Cargo.toml");

        let options = Arguments {
            find: test_spanned_string("Cargo.(.+)"),
            replace: ReplacementValue::String(Arc::new(test_spanned_string("Carga.$1"))),
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
