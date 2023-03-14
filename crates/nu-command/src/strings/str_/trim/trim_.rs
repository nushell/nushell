use crate::input_handler::{operate, CmdArgument};
use nu_engine::CallExt;
use nu_protocol::{
    ast::{Call, CellPath},
    engine::{Command, EngineState, Stack},
    Example, PipelineData, ShellError, Signature, Span, Spanned, SyntaxShape, Type, Value,
};

#[derive(Clone)]
pub struct SubCommand;

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

impl Command for SubCommand {
    fn name(&self) -> &str {
        "str trim"
    }

    fn signature(&self) -> Signature {
        Signature::build("str trim")
            .input_output_types(vec![(Type::String, Type::String)])
            .vectorizes_over_list(true)
            .rest(
                "rest",
                SyntaxShape::CellPath,
                "For a data structure input, trim strings at the given cell paths",
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
    }
    fn usage(&self) -> &str {
        "Trim whitespace or specific character."
    }

    fn search_terms(&self) -> Vec<&str> {
        vec!["whitespace", "strip", "lstrip", "rstrip"]
    }

    fn run(
        &self,
        engine_state: &EngineState,
        stack: &mut Stack,
        call: &Call,
        input: PipelineData,
    ) -> Result<PipelineData, ShellError> {
        let character = call.get_flag::<Spanned<String>>(engine_state, stack, "char")?;
        let to_trim = match character.as_ref() {
            Some(v) => {
                if v.item.chars().count() > 1 {
                    return Err(ShellError::GenericError(
                        "Trim only works with single character".into(),
                        "needs single character".into(),
                        Some(v.span),
                        None,
                        Vec::new(),
                    ));
                }
                v.item.chars().next()
            }
            None => None,
        };
        let cell_paths: Vec<CellPath> = call.rest(engine_state, stack, 0)?;
        let cell_paths = (!cell_paths.is_empty()).then_some(cell_paths);
        let mode = match cell_paths {
            None => ActionMode::Global,
            Some(_) => ActionMode::Local,
        };

        let left = call.has_flag("left");
        let right = call.has_flag("right");
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
        operate(action, args, input, call.head, engine_state.ctrlc.clone())
    }

    fn examples(&self) -> Vec<Example> {
        vec![
            Example {
                description: "Trim whitespace",
                example: "'Nu shell ' | str trim",
                result: Some(Value::test_string("Nu shell")),
            },
            Example {
                description: "Trim a specific character",
                example: "'=== Nu shell ===' | str trim -c '=' | str trim",
                result: Some(Value::test_string("Nu shell")),
            },
            Example {
                description: "Trim whitespace from the beginning of string",
                example: "' Nu shell ' | str trim -l",
                result: Some(Value::test_string("Nu shell ")),
            },
            Example {
                description: "Trim a specific character",
                example: "'=== Nu shell ===' | str trim -c '='",
                result: Some(Value::test_string(" Nu shell ")),
            },
            Example {
                description: "Trim whitespace from the end of string",
                example: "' Nu shell ' | str trim -r",
                result: Some(Value::test_string(" Nu shell")),
            },
            Example {
                description: "Trim a specific character",
                example: "'=== Nu shell ===' | str trim -r -c '='",
                result: Some(Value::test_string("=== Nu shell ")),
            },
        ]
    }
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
        Value::String { val: s, .. } => Value::String {
            val: trim(s, char_, trim_side),
            span: head,
        },
        // Propagate errors by explicitly matching them before the final case.
        Value::Error { .. } => input.clone(),
        other => match mode {
            ActionMode::Global => match other {
                Value::Record { cols, vals, span } => {
                    let new_vals = vals.iter().map(|v| action(v, arg, head)).collect();

                    Value::Record {
                        cols: cols.to_vec(),
                        vals: new_vals,
                        span: *span,
                    }
                }
                Value::List { vals, span } => {
                    let new_vals = vals.iter().map(|v| action(v, arg, head)).collect();

                    Value::List {
                        vals: new_vals,
                        span: *span,
                    }
                }
                _ => input.clone(),
            },
            ActionMode::Local => {
                Value::Error {
                    error: Box::new(ShellError::UnsupportedInput(
                        "Only string values are supported".into(),
                        format!("input type: {:?}", other.get_type()),
                        head,
                        // This line requires the Value::Error match above.
                        other.expect_span(),
                    )),
                }
            }
        },
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

        test_examples(SubCommand {})
    }

    fn make_record(cols: Vec<&str>, vals: Vec<&str>) -> Value {
        Value::Record {
            cols: cols.iter().map(|x| x.to_string()).collect(),
            vals: vals
                .iter()
                .map(|x| Value::test_string(x.to_string()))
                .collect(),
            span: Span::test_data(),
        }
    }

    fn make_list(vals: Vec<&str>) -> Value {
        Value::List {
            vals: vals
                .iter()
                .map(|x| Value::test_string(x.to_string()))
                .collect(),
            span: Span::test_data(),
        }
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
        let row = Value::List {
            vals: vec![
                Value::test_string("  a  "),
                Value::test_int(65),
                Value::test_string(" d"),
            ],
            span: Span::test_data(),
        };
        let expected = Value::List {
            vals: vec![
                Value::test_string("a  "),
                Value::test_int(65),
                Value::test_string("d"),
            ],
            span: Span::test_data(),
        };

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
        let row = Value::List {
            vals: vec![
                Value::test_string("  a  "),
                Value::test_int(65),
                Value::test_string(" d"),
            ],
            span: Span::test_data(),
        };
        let expected = Value::List {
            vals: vec![
                Value::test_string("  a"),
                Value::test_int(65),
                Value::test_string(" d"),
            ],
            span: Span::test_data(),
        };
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
