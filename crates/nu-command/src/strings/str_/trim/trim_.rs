use nu_cmd_base::input_handler::{CmdArgument, operate};
use nu_engine::command_prelude::*;

#[derive(Clone)]
pub struct StrTrim;

struct Arguments {
    to_trim: Option<char>,
    trim_side: TrimSide,
    cell_paths: Option<Vec<CellPath>>,
    mode: ActionMode,
}

impl CmdArgument for Arguments {
    fn take_cell_paths(&mut self) -> Option<Vec<CellPath>> {
        self.cell_paths.take()
    }
}

pub enum TrimSide {
    Left,
    Right,
    Both,
}

impl Command for StrTrim {
    fn name(&self) -> &str {
        "str trim"
    }

    fn signature(&self) -> Signature {
        Signature::build("str trim")
            .input_output_types(vec![
                (Type::String, Type::String),
                (
                    Type::List(Box::new(Type::String)),
                    Type::List(Box::new(Type::String)),
                ),
                (Type::table(), Type::table()),
                (Type::record(), Type::record()),
            ])
            .allow_variants_without_examples(true)
            .rest(
                "rest",
                SyntaxShape::CellPath,
                "For a data structure input, trim strings at the given cell paths.",
            )
            .named(
                "char",
                SyntaxShape::String,
                "character to trim (default: whitespace)",
                Some('c'),
            )
            .switch(
                "left",
                "trims characters only from the beginning of the string",
                Some('l'),
            )
            .switch(
                "right",
                "trims characters only from the end of the string",
                Some('r'),
            )
            .category(Category::Strings)
    }
    fn description(&self) -> &str {
        "Trim whitespace or specific character."
    }

    fn search_terms(&self) -> Vec<&str> {
        vec!["whitespace", "strip", "lstrip", "rstrip"]
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
        let character = call.get_flag::<Spanned<String>>(engine_state, stack, "char")?;
        let cell_paths: Vec<CellPath> = call.rest(engine_state, stack, 0)?;
        let left = call.has_flag(engine_state, stack, "left")?;
        let right = call.has_flag(engine_state, stack, "right")?;
        run(
            character,
            cell_paths,
            (left, right),
            call,
            input,
            engine_state,
        )
    }

    fn run_const(
        &self,
        working_set: &StateWorkingSet,
        call: &Call,
        input: PipelineData,
    ) -> Result<PipelineData, ShellError> {
        let character = call.get_flag_const::<Spanned<String>>(working_set, "char")?;
        let cell_paths: Vec<CellPath> = call.rest_const(working_set, 0)?;
        let left = call.has_flag_const(working_set, "left")?;
        let right = call.has_flag_const(working_set, "right")?;
        run(
            character,
            cell_paths,
            (left, right),
            call,
            input,
            working_set.permanent(),
        )
    }

    fn examples(&self) -> Vec<Example<'_>> {
        vec![
            Example {
                description: "Trim whitespace",
                example: "'Nu shell ' | str trim",
                result: Some(Value::test_string("Nu shell")),
            },
            Example {
                description: "Trim a specific character (not the whitespace)",
                example: "'=== Nu shell ===' | str trim --char '='",
                result: Some(Value::test_string(" Nu shell ")),
            },
            Example {
                description: "Trim whitespace from the beginning of string",
                example: "' Nu shell ' | str trim --left",
                result: Some(Value::test_string("Nu shell ")),
            },
            Example {
                description: "Trim whitespace from the end of string",
                example: "' Nu shell ' | str trim --right",
                result: Some(Value::test_string(" Nu shell")),
            },
            Example {
                description: "Trim a specific character only from the end of the string",
                example: "'=== Nu shell ===' | str trim --right --char '='",
                result: Some(Value::test_string("=== Nu shell ")),
            },
        ]
    }
}

fn run(
    character: Option<Spanned<String>>,
    cell_paths: Vec<CellPath>,
    (left, right): (bool, bool),
    call: &Call,
    input: PipelineData,
    engine_state: &EngineState,
) -> Result<PipelineData, ShellError> {
    let to_trim = match character.as_ref() {
        Some(v) => {
            if v.item.chars().count() > 1 {
                return Err(ShellError::GenericError {
                    error: "Trim only works with single character".into(),
                    msg: "needs single character".into(),
                    span: Some(v.span),
                    help: None,
                    inner: vec![],
                });
            }
            v.item.chars().next()
        }
        None => None,
    };

    let cell_paths = (!cell_paths.is_empty()).then_some(cell_paths);
    let mode = match cell_paths {
        None => ActionMode::Global,
        Some(_) => ActionMode::Local,
    };

    let trim_side = match (left, right) {
        (true, true) => TrimSide::Both,
        (true, false) => TrimSide::Left,
        (false, true) => TrimSide::Right,
        (false, false) => TrimSide::Both,
    };

    let args = Arguments {
        to_trim,
        trim_side,
        cell_paths,
        mode,
    };
    operate(action, args, input, call.head, engine_state.signals())
}

#[derive(Debug, Copy, Clone)]
pub enum ActionMode {
    Local,
    Global,
}

fn action(input: &Value, arg: &Arguments, head: Span) -> Value {
    let char_ = arg.to_trim;
    let trim_side = &arg.trim_side;
    let mode = &arg.mode;
    match input {
        Value::String { val: s, .. } => Value::string(trim(s, char_, trim_side), head),
        // Propagate errors by explicitly matching them before the final case.
        Value::Error { .. } => input.clone(),
        other => {
            let span = other.span();

            match mode {
                ActionMode::Global => match other {
                    Value::Record { val: record, .. } => {
                        let new_record = record
                            .iter()
                            .map(|(k, v)| (k.clone(), action(v, arg, head)))
                            .collect();

                        Value::record(new_record, span)
                    }
                    Value::List { vals, .. } => {
                        let new_vals = vals.iter().map(|v| action(v, arg, head)).collect();

                        Value::list(new_vals, span)
                    }
                    _ => input.clone(),
                },
                ActionMode::Local => Value::error(
                    ShellError::UnsupportedInput {
                        msg: "Only string values are supported".into(),
                        input: format!("input type: {:?}", other.get_type()),
                        msg_span: head,
                        input_span: other.span(),
                    },
                    head,
                ),
            }
        }
    }
}

fn trim(s: &str, char_: Option<char>, trim_side: &TrimSide) -> String {
    let delimiters = match char_ {
        Some(c) => vec![c],
        // Trying to make this trim work like rust default trim()
        // which uses is_whitespace() as a default
        None => vec![
            ' ',    // space
            '\x09', // horizontal tab
            '\x0A', // new line, line feed
            '\x0B', // vertical tab
            '\x0C', // form feed, new page
            '\x0D', // carriage return
        ], //whitespace
    };

    match trim_side {
        TrimSide::Left => s.trim_start_matches(&delimiters[..]).to_string(),
        TrimSide::Right => s.trim_end_matches(&delimiters[..]).to_string(),
        TrimSide::Both => s.trim_matches(&delimiters[..]).to_string(),
    }
}

#[cfg(test)]
mod tests {
    use crate::strings::str_::trim::trim_::*;
    use nu_protocol::{Span, Value};

    #[test]
    fn test_examples() {
        use crate::test_examples;

        test_examples(StrTrim {})
    }

    fn make_record(cols: Vec<&str>, vals: Vec<&str>) -> Value {
        Value::test_record(
            cols.into_iter()
                .zip(vals)
                .map(|(col, val)| (col.to_owned(), Value::test_string(val)))
                .collect(),
        )
    }

    fn make_list(vals: Vec<&str>) -> Value {
        Value::list(
            vals.iter()
                .map(|x| Value::test_string(x.to_string()))
                .collect(),
            Span::test_data(),
        )
    }

    #[test]
    fn trims() {
        let word = Value::test_string("andres ");
        let expected = Value::test_string("andres");

        let args = Arguments {
            to_trim: None,
            trim_side: TrimSide::Both,
            cell_paths: None,
            mode: ActionMode::Local,
        };
        let actual = action(&word, &args, Span::test_data());
        assert_eq!(actual, expected);
    }

    #[test]
    fn trims_global() {
        let word = Value::test_string(" global   ");
        let expected = Value::test_string("global");
        let args = Arguments {
            to_trim: None,
            trim_side: TrimSide::Both,
            cell_paths: None,
            mode: ActionMode::Global,
        };
        let actual = action(&word, &args, Span::test_data());
        assert_eq!(actual, expected);
    }

    #[test]
    fn global_trim_ignores_numbers() {
        let number = Value::test_int(2020);
        let expected = Value::test_int(2020);
        let args = Arguments {
            to_trim: None,
            trim_side: TrimSide::Both,
            cell_paths: None,
            mode: ActionMode::Global,
        };

        let actual = action(&number, &args, Span::test_data());
        assert_eq!(actual, expected);
    }

    #[test]
    fn global_trim_row() {
        let row = make_record(vec!["a", "b"], vec!["    c ", "  d   "]);
        // ["a".to_string() => string("    c "), " b ".to_string() => string("  d   ")];
        let expected = make_record(vec!["a", "b"], vec!["c", "d"]);

        let args = Arguments {
            to_trim: None,
            trim_side: TrimSide::Both,
            cell_paths: None,
            mode: ActionMode::Global,
        };
        let actual = action(&row, &args, Span::test_data());
        assert_eq!(actual, expected);
    }

    #[test]
    fn global_trim_table() {
        let row = make_list(vec!["  a  ", "d"]);
        let expected = make_list(vec!["a", "d"]);

        let args = Arguments {
            to_trim: None,
            trim_side: TrimSide::Both,
            cell_paths: None,
            mode: ActionMode::Global,
        };
        let actual = action(&row, &args, Span::test_data());
        assert_eq!(actual, expected);
    }

    #[test]
    fn trims_custom_character_both_ends() {
        let word = Value::test_string("!#andres#!");
        let expected = Value::test_string("#andres#");

        let args = Arguments {
            to_trim: Some('!'),
            trim_side: TrimSide::Both,
            cell_paths: None,
            mode: ActionMode::Local,
        };
        let actual = action(&word, &args, Span::test_data());
        assert_eq!(actual, expected);
    }

    #[test]
    fn trims_whitespace_from_left() {
        let word = Value::test_string(" andres ");
        let expected = Value::test_string("andres ");

        let args = Arguments {
            to_trim: None,
            trim_side: TrimSide::Left,
            cell_paths: None,
            mode: ActionMode::Local,
        };
        let actual = action(&word, &args, Span::test_data());
        assert_eq!(actual, expected);
    }

    #[test]
    fn global_trim_left_ignores_numbers() {
        let number = Value::test_int(2020);
        let expected = Value::test_int(2020);

        let args = Arguments {
            to_trim: None,
            trim_side: TrimSide::Left,
            cell_paths: None,
            mode: ActionMode::Global,
        };
        let actual = action(&number, &args, Span::test_data());
        assert_eq!(actual, expected);
    }

    #[test]
    fn trims_left_global() {
        let word = Value::test_string(" global   ");
        let expected = Value::test_string("global   ");

        let args = Arguments {
            to_trim: None,
            trim_side: TrimSide::Left,
            cell_paths: None,
            mode: ActionMode::Global,
        };
        let actual = action(&word, &args, Span::test_data());
        assert_eq!(actual, expected);
    }

    #[test]
    fn global_trim_left_row() {
        let row = make_record(vec!["a", "b"], vec!["    c ", "  d   "]);
        let expected = make_record(vec!["a", "b"], vec!["c ", "d   "]);

        let args = Arguments {
            to_trim: None,
            trim_side: TrimSide::Left,
            cell_paths: None,
            mode: ActionMode::Global,
        };
        let actual = action(&row, &args, Span::test_data());
        assert_eq!(actual, expected);
    }

    #[test]
    fn global_trim_left_table() {
        let row = Value::list(
            vec![
                Value::test_string("  a  "),
                Value::test_int(65),
                Value::test_string(" d"),
            ],
            Span::test_data(),
        );
        let expected = Value::list(
            vec![
                Value::test_string("a  "),
                Value::test_int(65),
                Value::test_string("d"),
            ],
            Span::test_data(),
        );

        let args = Arguments {
            to_trim: None,
            trim_side: TrimSide::Left,
            cell_paths: None,
            mode: ActionMode::Global,
        };
        let actual = action(&row, &args, Span::test_data());
        assert_eq!(actual, expected);
    }

    #[test]
    fn trims_custom_chars_from_left() {
        let word = Value::test_string("!!! andres !!!");
        let expected = Value::test_string(" andres !!!");

        let args = Arguments {
            to_trim: Some('!'),
            trim_side: TrimSide::Left,
            cell_paths: None,
            mode: ActionMode::Local,
        };
        let actual = action(&word, &args, Span::test_data());
        assert_eq!(actual, expected);
    }
    #[test]
    fn trims_whitespace_from_right() {
        let word = Value::test_string(" andres ");
        let expected = Value::test_string(" andres");

        let args = Arguments {
            to_trim: None,
            trim_side: TrimSide::Right,
            cell_paths: None,
            mode: ActionMode::Local,
        };
        let actual = action(&word, &args, Span::test_data());
        assert_eq!(actual, expected);
    }

    #[test]
    fn trims_right_global() {
        let word = Value::test_string(" global   ");
        let expected = Value::test_string(" global");
        let args = Arguments {
            to_trim: None,
            trim_side: TrimSide::Right,
            cell_paths: None,
            mode: ActionMode::Global,
        };
        let actual = action(&word, &args, Span::test_data());
        assert_eq!(actual, expected);
    }

    #[test]
    fn global_trim_right_ignores_numbers() {
        let number = Value::test_int(2020);
        let expected = Value::test_int(2020);
        let args = Arguments {
            to_trim: None,
            trim_side: TrimSide::Right,
            cell_paths: None,
            mode: ActionMode::Global,
        };
        let actual = action(&number, &args, Span::test_data());
        assert_eq!(actual, expected);
    }

    #[test]
    fn global_trim_right_row() {
        let row = make_record(vec!["a", "b"], vec!["    c ", "  d   "]);
        let expected = make_record(vec!["a", "b"], vec!["    c", "  d"]);
        let args = Arguments {
            to_trim: None,
            trim_side: TrimSide::Right,
            cell_paths: None,
            mode: ActionMode::Global,
        };
        let actual = action(&row, &args, Span::test_data());
        assert_eq!(actual, expected);
    }

    #[test]
    fn global_trim_right_table() {
        let row = Value::list(
            vec![
                Value::test_string("  a  "),
                Value::test_int(65),
                Value::test_string(" d"),
            ],
            Span::test_data(),
        );
        let expected = Value::list(
            vec![
                Value::test_string("  a"),
                Value::test_int(65),
                Value::test_string(" d"),
            ],
            Span::test_data(),
        );
        let args = Arguments {
            to_trim: None,
            trim_side: TrimSide::Right,
            cell_paths: None,
            mode: ActionMode::Global,
        };
        let actual = action(&row, &args, Span::test_data());
        assert_eq!(actual, expected);
    }

    #[test]
    fn trims_custom_chars_from_right() {
        let word = Value::test_string("#@! andres !@#");
        let expected = Value::test_string("#@! andres !@");

        let args = Arguments {
            to_trim: Some('#'),
            trim_side: TrimSide::Right,
            cell_paths: None,
            mode: ActionMode::Local,
        };
        let actual = action(&word, &args, Span::test_data());
        assert_eq!(actual, expected);
    }
}
