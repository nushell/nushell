use nu_engine::CallExt;
use nu_protocol::{
    ast::{Call, CellPath},
    engine::{Command, EngineState, Stack},
    Example, PipelineData, ShellError, Signature, Span, Spanned, SyntaxShape, Value,
};

#[derive(Clone)]
pub struct SubCommand;

struct Arguments {
    character: Option<Spanned<String>>,
    column_paths: Vec<CellPath>,
}

#[derive(Default, Debug, Copy, Clone)]
pub struct ClosureFlags {
    all_flag: bool,
    left_trim: bool,
    right_trim: bool,
    format_flag: bool,
    both_flag: bool,
}

impl Command for SubCommand {
    fn name(&self) -> &str {
        "str trim"
    }

    fn signature(&self) -> Signature {
        Signature::build("str trim")
            .rest(
                "rest",
                SyntaxShape::CellPath,
                "optionally trim text by column paths",
            )
            .named(
                "char",
                SyntaxShape::String,
                "character to trim (default: whitespace)",
                Some('c'),
            )
            .switch(
                "left",
                "trims characters only from the beginning of the string (default: whitespace)",
                Some('l'),
            )
            .switch(
                "right",
                "trims characters only from the end of the string (default: whitespace)",
                Some('r'),
            )
            .switch(
                "all",
                "trims all characters from both sides of the string *and* in the middle (default: whitespace)",
                Some('a'),
            )
            .switch("both", "trims all characters from left and right side of the string (default: whitespace)", Some('b'))
            .switch("format", "trims spaces replacing multiple characters with singles in the middle (default: whitespace)", Some('f'))
    }
    fn usage(&self) -> &str {
        "Trim whitespace or specific character"
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
    ) -> Result<nu_protocol::PipelineData, nu_protocol::ShellError> {
        operate(engine_state, stack, call, input, &trim)
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
                description: "Trim all characters",
                example: "' Nu   shell ' | str trim -a",
                result: Some(Value::test_string("Nushell")),
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

pub fn operate<F>(
    engine_state: &EngineState,
    stack: &mut Stack,
    call: &Call,
    input: PipelineData,
    trim_operation: &'static F,
) -> Result<nu_protocol::PipelineData, nu_protocol::ShellError>
where
    F: Fn(&str, Option<char>, &ClosureFlags) -> String + Send + Sync + 'static,
{
    let head = call.head;
    let (options, closure_flags, input) = (
        Arguments {
            character: call.get_flag(engine_state, stack, "char")?,
            column_paths: call.rest(engine_state, stack, 0)?,
        },
        ClosureFlags {
            all_flag: call.has_flag("all"),
            left_trim: call.has_flag("left"),
            right_trim: call.has_flag("right"),
            format_flag: call.has_flag("format"),
            both_flag: call.has_flag("both")
                || (!call.has_flag("all")
                    && !call.has_flag("left")
                    && !call.has_flag("right")
                    && !call.has_flag("format")), // this is the case if no flags are provided
        },
        input,
    );
    let to_trim = match options.character.as_ref() {
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

    input.map(
        move |v| {
            if options.column_paths.is_empty() {
                action(
                    &v,
                    head,
                    to_trim,
                    &closure_flags,
                    &trim_operation,
                    ActionMode::Global,
                )
            } else {
                let mut ret = v;
                for path in &options.column_paths {
                    let r = ret.update_cell_path(
                        &path.members,
                        Box::new(move |old| {
                            action(
                                old,
                                head,
                                to_trim,
                                &closure_flags,
                                &trim_operation,
                                ActionMode::Local,
                            )
                        }),
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

#[derive(Debug, Copy, Clone)]
pub enum ActionMode {
    Local,
    Global,
}

pub fn action<F>(
    input: &Value,
    head: Span,
    char_: Option<char>,
    closure_flags: &ClosureFlags,
    trim_operation: &F,
    mode: ActionMode,
) -> Value
where
    F: Fn(&str, Option<char>, &ClosureFlags) -> String + Send + Sync + 'static,
{
    match input {
        Value::String { val: s, .. } => Value::String {
            val: trim_operation(s, char_, closure_flags),
            span: head,
        },
        other => match mode {
            ActionMode::Global => match other {
                Value::Record { cols, vals, span } => {
                    let new_vals = vals
                        .iter()
                        .map(|v| action(v, head, char_, closure_flags, trim_operation, mode))
                        .collect();

                    Value::Record {
                        cols: cols.to_vec(),
                        vals: new_vals,
                        span: *span,
                    }
                }
                Value::List { vals, span } => {
                    let new_vals = vals
                        .iter()
                        .map(|v| action(v, head, char_, closure_flags, trim_operation, mode))
                        .collect();

                    Value::List {
                        vals: new_vals,
                        span: *span,
                    }
                }
                _ => input.clone(),
            },
            ActionMode::Local => {
                let got = format!("Input must be a string. Found {}", other.get_type());
                Value::Error {
                    error: ShellError::UnsupportedInput(got, head),
                }
            }
        },
    }
}

fn trim(s: &str, char_: Option<char>, closure_flags: &ClosureFlags) -> String {
    let ClosureFlags {
        left_trim,
        right_trim,
        all_flag,
        both_flag,
        format_flag,
    } = closure_flags;
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

    if *left_trim {
        s.trim_start_matches(&delimiters[..]).to_string()
    } else if *right_trim {
        s.trim_end_matches(&delimiters[..]).to_string()
    } else if *all_flag {
        s.split(&delimiters[..])
            .filter(|s| !s.is_empty())
            .collect::<String>()
    } else if *both_flag {
        s.trim_matches(&delimiters[..]).to_string()
    } else if *format_flag {
        // The idea here is to use regex to go through these delimiters and
        // where there are multiple, replace them with singles

        // create our return string which is a copy of the original string
        let mut return_string = String::from(s);
        // Iterate through the delimiters replacing them with regex friendly names
        for r in &delimiters {
            let reg = match r {
                ' ' => r"\s".to_string(),
                '\x09' => r"\t".to_string(),
                '\x0A' => r"\n".to_string(),
                '\x0B' => r"\v".to_string(),
                '\x0C' => r"\f".to_string(),
                '\x0D' => r"\r".to_string(),
                _ => format!(r"\{}", r),
            };
            // create a regex string that looks for 2 or more of each of these characters
            let re_str = format!("{}{{2,}}", reg);
            // create the regex
            let re = regex::Regex::new(&re_str).expect("Error creating regular expression");
            // replace all mutliple occurances with single occurences represented by r
            let new_str = re.replace_all(&return_string, r.to_string());
            // update the return string so the next loop has the latest changes
            return_string = new_str.to_string();
        }
        // for good measure, trim_matches, which gets the start and end
        // theoretically we shouldn't have to do this but from my testing, we do.
        return_string.trim_matches(&delimiters[..]).to_string()
    } else {
        s.trim().to_string()
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
                .map(|x| Value::String {
                    val: x.to_string(),
                    span: Span::test_data(),
                })
                .collect(),
            span: Span::test_data(),
        }
    }

    fn make_list(vals: Vec<&str>) -> Value {
        Value::List {
            vals: vals
                .iter()
                .map(|x| Value::String {
                    val: x.to_string(),
                    span: Span::test_data(),
                })
                .collect(),
            span: Span::test_data(),
        }
    }

    #[test]
    fn trims() {
        let word = Value::test_string("andres ");
        let expected = Value::test_string("andres");
        let closure_flags = ClosureFlags {
            both_flag: true,
            ..Default::default()
        };

        let actual = action(
            &word,
            Span::test_data(),
            None,
            &closure_flags,
            &trim,
            ActionMode::Local,
        );
        assert_eq!(actual, expected);
    }

    #[test]
    fn trims_global() {
        let word = Value::test_string(" global   ");
        let expected = Value::test_string("global");
        let closure_flags = ClosureFlags {
            both_flag: true,
            ..Default::default()
        };

        let actual = action(
            &word,
            Span::test_data(),
            None,
            &closure_flags,
            &trim,
            ActionMode::Global,
        );
        assert_eq!(actual, expected);
    }

    #[test]
    fn global_trim_ignores_numbers() {
        let number = Value::test_int(2020);
        let expected = Value::test_int(2020);
        let closure_flags = ClosureFlags {
            both_flag: true,
            ..Default::default()
        };

        let actual = action(
            &number,
            Span::test_data(),
            None,
            &closure_flags,
            &trim,
            ActionMode::Global,
        );
        assert_eq!(actual, expected);
    }

    #[test]
    fn global_trim_row() {
        let row = make_record(vec!["a", "b"], vec!["    c ", "  d   "]);
        // ["a".to_string() => string("    c "), " b ".to_string() => string("  d   ")];
        let expected = make_record(vec!["a", "b"], vec!["c", "d"]);
        let closure_flags = ClosureFlags {
            both_flag: true,
            ..Default::default()
        };

        let actual = action(
            &row,
            Span::test_data(),
            None,
            &closure_flags,
            &trim,
            ActionMode::Global,
        );
        assert_eq!(actual, expected);
    }

    #[test]
    fn global_trim_table() {
        let row = make_list(vec!["  a  ", "d"]);
        let expected = make_list(vec!["a", "d"]);
        let closure_flags = ClosureFlags {
            both_flag: true,
            ..Default::default()
        };

        let actual = action(
            &row,
            Span::test_data(),
            None,
            &closure_flags,
            &trim,
            ActionMode::Global,
        );
        assert_eq!(actual, expected);
    }

    #[test]
    fn trims_custom_character_both_ends() {
        let word = Value::test_string("!#andres#!");
        let expected = Value::test_string("#andres#");
        let closure_flags = ClosureFlags {
            both_flag: true,
            ..Default::default()
        };

        let actual = action(
            &word,
            Span::test_data(),
            Some('!'),
            &closure_flags,
            &trim,
            ActionMode::Local,
        );
        assert_eq!(actual, expected);
    }
    #[test]
    fn trims_all_white_space() {
        let word = Value::test_string(" Value1 a lot  of  spaces ");
        let expected = Value::test_string("Value1alotofspaces");
        let closure_flags = ClosureFlags {
            all_flag: true,
            ..Default::default()
        };

        let actual = action(
            &word,
            Span::test_data(),
            Some(' '),
            &closure_flags,
            &trim,
            ActionMode::Local,
        );
        assert_eq!(actual, expected);
    }

    #[test]
    fn global_trims_row_all_white_space() {
        let row = make_record(
            vec!["a", "b"],
            vec!["    nu    shell ", "  b c   d     e  "],
        );
        let expected = make_record(vec!["a", "b"], vec!["nushell", "bcde"]);

        let closure_flags = ClosureFlags {
            all_flag: true,
            ..Default::default()
        };

        let actual = action(
            &row,
            Span::test_data(),
            None,
            &closure_flags,
            &trim,
            ActionMode::Global,
        );
        assert_eq!(actual, expected);
    }

    #[test]
    fn global_trims_table_all_white_space() {
        let row = Value::List {
            vals: vec![
                Value::String {
                    val: "  nu      shell   ".to_string(),
                    span: Span::test_data(),
                },
                Value::Int {
                    val: 65,
                    span: Span::test_data(),
                },
                Value::String {
                    val: "  d".to_string(),
                    span: Span::test_data(),
                },
            ],
            span: Span::test_data(),
        };
        let expected = Value::List {
            vals: vec![
                Value::String {
                    val: "nushell".to_string(),
                    span: Span::test_data(),
                },
                Value::Int {
                    val: 65,
                    span: Span::test_data(),
                },
                Value::String {
                    val: "d".to_string(),
                    span: Span::test_data(),
                },
            ],
            span: Span::test_data(),
        };

        let closure_flags = ClosureFlags {
            all_flag: true,
            ..Default::default()
        };

        let actual = action(
            &row,
            Span::test_data(),
            None,
            &closure_flags,
            &trim,
            ActionMode::Global,
        );
        assert_eq!(actual, expected);
    }

    #[test]
    fn trims_all_custom_character() {
        let word = Value::test_string(".Value1.a.lot..of...dots.");
        let expected = Value::test_string("Value1alotofdots");
        let closure_flags = ClosureFlags {
            all_flag: true,
            ..Default::default()
        };

        let actual = action(
            &word,
            Span::test_data(),
            Some('.'),
            &closure_flags,
            &trim,
            ActionMode::Local,
        );
        assert_eq!(actual, expected);
    }

    #[test]
    fn global_trims_row_all_custom_character() {
        let row = make_record(vec!["a", "b"], vec!["!!!!nu!!shell!!!", "!!b!c!!d!e!!"]);
        let expected = make_record(vec!["a", "b"], vec!["nushell", "bcde"]);
        let closure_flags = ClosureFlags {
            all_flag: true,
            ..Default::default()
        };

        let actual = action(
            &row,
            Span::test_data(),
            Some('!'),
            &closure_flags,
            &trim,
            ActionMode::Global,
        );
        assert_eq!(actual, expected);
    }

    #[test]
    fn global_trims_table_all_custom_character() {
        let row = Value::List {
            vals: vec![
                Value::String {
                    val: "##nu####shell##".to_string(),
                    span: Span::test_data(),
                },
                Value::Int {
                    val: 65,
                    span: Span::test_data(),
                },
                Value::String {
                    val: "#d".to_string(),
                    span: Span::test_data(),
                },
            ],
            span: Span::test_data(),
        };
        let expected = Value::List {
            vals: vec![
                Value::String {
                    val: "nushell".to_string(),
                    span: Span::test_data(),
                },
                Value::Int {
                    val: 65,
                    span: Span::test_data(),
                },
                Value::String {
                    val: "d".to_string(),
                    span: Span::test_data(),
                },
            ],
            span: Span::test_data(),
        };

        let closure_flags = ClosureFlags {
            all_flag: true,
            ..Default::default()
        };

        let actual = action(
            &row,
            Span::test_data(),
            Some('#'),
            &closure_flags,
            &trim,
            ActionMode::Global,
        );
        assert_eq!(actual, expected);
    }

    #[test]
    fn trims_whitespace_from_left() {
        let word = Value::test_string(" andres ");
        let expected = Value::test_string("andres ");
        let closure_flags = ClosureFlags {
            left_trim: true,
            ..Default::default()
        };

        let actual = action(
            &word,
            Span::test_data(),
            None,
            &closure_flags,
            &trim,
            ActionMode::Local,
        );
        assert_eq!(actual, expected);
    }

    #[test]
    fn global_trim_left_ignores_numbers() {
        let number = Value::test_int(2020);
        let expected = Value::test_int(2020);
        let closure_flags = ClosureFlags {
            left_trim: true,
            ..Default::default()
        };

        let actual = action(
            &number,
            Span::test_data(),
            None,
            &closure_flags,
            &trim,
            ActionMode::Global,
        );
        assert_eq!(actual, expected);
    }

    #[test]
    fn trims_left_global() {
        let word = Value::test_string(" global   ");
        let expected = Value::test_string("global   ");
        let closure_flags = ClosureFlags {
            left_trim: true,
            ..Default::default()
        };

        let actual = action(
            &word,
            Span::test_data(),
            None,
            &closure_flags,
            &trim,
            ActionMode::Global,
        );
        assert_eq!(actual, expected);
    }

    #[test]
    fn global_trim_left_row() {
        let row = make_record(vec!["a", "b"], vec!["    c ", "  d   "]);
        let expected = make_record(vec!["a", "b"], vec!["c ", "d   "]);
        let closure_flags = ClosureFlags {
            left_trim: true,
            ..Default::default()
        };

        let actual = action(
            &row,
            Span::test_data(),
            None,
            &closure_flags,
            &trim,
            ActionMode::Global,
        );
        assert_eq!(actual, expected);
    }

    #[test]
    fn global_trim_left_table() {
        let row = Value::List {
            vals: vec![
                Value::String {
                    val: "  a  ".to_string(),
                    span: Span::test_data(),
                },
                Value::Int {
                    val: 65,
                    span: Span::test_data(),
                },
                Value::String {
                    val: " d".to_string(),
                    span: Span::test_data(),
                },
            ],
            span: Span::test_data(),
        };
        let expected = Value::List {
            vals: vec![
                Value::String {
                    val: "a  ".to_string(),
                    span: Span::test_data(),
                },
                Value::Int {
                    val: 65,
                    span: Span::test_data(),
                },
                Value::String {
                    val: "d".to_string(),
                    span: Span::test_data(),
                },
            ],
            span: Span::test_data(),
        };

        let closure_flags = ClosureFlags {
            left_trim: true,
            ..Default::default()
        };

        let actual = action(
            &row,
            Span::test_data(),
            None,
            &closure_flags,
            &trim,
            ActionMode::Global,
        );
        assert_eq!(actual, expected);
    }

    #[test]
    fn trims_custom_chars_from_left() {
        let word = Value::test_string("!!! andres !!!");
        let expected = Value::test_string(" andres !!!");
        let closure_flags = ClosureFlags {
            left_trim: true,
            ..Default::default()
        };

        let actual = action(
            &word,
            Span::test_data(),
            Some('!'),
            &closure_flags,
            &trim,
            ActionMode::Local,
        );
        assert_eq!(actual, expected);
    }
    #[test]
    fn trims_whitespace_from_right() {
        let word = Value::test_string(" andres ");
        let expected = Value::test_string(" andres");
        let closure_flags = ClosureFlags {
            right_trim: true,
            ..Default::default()
        };

        let actual = action(
            &word,
            Span::test_data(),
            None,
            &closure_flags,
            &trim,
            ActionMode::Local,
        );
        assert_eq!(actual, expected);
    }

    #[test]
    fn trims_right_global() {
        let word = Value::test_string(" global   ");
        let expected = Value::test_string(" global");
        let closure_flags = ClosureFlags {
            right_trim: true,
            ..Default::default()
        };

        let actual = action(
            &word,
            Span::test_data(),
            None,
            &closure_flags,
            &trim,
            ActionMode::Global,
        );
        assert_eq!(actual, expected);
    }

    #[test]
    fn global_trim_right_ignores_numbers() {
        let number = Value::test_int(2020);
        let expected = Value::test_int(2020);
        let closure_flags = ClosureFlags {
            right_trim: true,
            ..Default::default()
        };

        let actual = action(
            &number,
            Span::test_data(),
            None,
            &closure_flags,
            &trim,
            ActionMode::Global,
        );
        assert_eq!(actual, expected);
    }

    #[test]
    fn global_trim_right_row() {
        let row = make_record(vec!["a", "b"], vec!["    c ", "  d   "]);
        let expected = make_record(vec!["a", "b"], vec!["    c", "  d"]);
        let closure_flags = ClosureFlags {
            right_trim: true,
            ..Default::default()
        };

        let actual = action(
            &row,
            Span::test_data(),
            None,
            &closure_flags,
            &trim,
            ActionMode::Global,
        );
        assert_eq!(actual, expected);
    }

    #[test]
    fn global_trim_right_table() {
        let row = Value::List {
            vals: vec![
                Value::String {
                    val: "  a  ".to_string(),
                    span: Span::test_data(),
                },
                Value::Int {
                    val: 65,
                    span: Span::test_data(),
                },
                Value::String {
                    val: " d".to_string(),
                    span: Span::test_data(),
                },
            ],
            span: Span::test_data(),
        };
        let expected = Value::List {
            vals: vec![
                Value::String {
                    val: "  a".to_string(),
                    span: Span::test_data(),
                },
                Value::Int {
                    val: 65,
                    span: Span::test_data(),
                },
                Value::String {
                    val: " d".to_string(),
                    span: Span::test_data(),
                },
            ],
            span: Span::test_data(),
        };

        let closure_flags = ClosureFlags {
            right_trim: true,
            ..Default::default()
        };

        let actual = action(
            &row,
            Span::test_data(),
            None,
            &closure_flags,
            &trim,
            ActionMode::Global,
        );
        assert_eq!(actual, expected);
    }

    #[test]
    fn trims_custom_chars_from_right() {
        let word = Value::test_string("#@! andres !@#");
        let expected = Value::test_string("#@! andres !@");
        let closure_flags = ClosureFlags {
            right_trim: true,
            ..Default::default()
        };

        let actual = action(
            &word,
            Span::test_data(),
            Some('#'),
            &closure_flags,
            &trim,
            ActionMode::Local,
        );
        assert_eq!(actual, expected);
    }

    #[test]
    fn trims_whitespace_format_flag() {
        let word = Value::test_string(" nushell    is     great ");
        let expected = Value::test_string("nushell is great");
        let closure_flags = ClosureFlags {
            format_flag: true,
            ..Default::default()
        };

        let actual = action(
            &word,
            Span::test_data(),
            Some(' '),
            &closure_flags,
            &trim,
            ActionMode::Local,
        );
        assert_eq!(actual, expected);
    }

    #[test]
    fn trims_format_flag_global() {
        let word = Value::test_string("global ");
        let expected = Value::test_string("global");
        let closure_flags = ClosureFlags {
            format_flag: true,
            ..Default::default()
        };

        let actual = action(
            &word,
            Span::test_data(),
            Some(' '),
            &closure_flags,
            &trim,
            ActionMode::Local,
        );
        assert_eq!(actual, expected);
    }
    #[test]
    fn global_trim_format_flag_ignores_numbers() {
        let number = Value::test_int(2020);
        let expected = Value::test_int(2020);
        let closure_flags = ClosureFlags {
            format_flag: true,
            ..Default::default()
        };

        let actual = action(
            &number,
            Span::test_data(),
            None,
            &closure_flags,
            &trim,
            ActionMode::Global,
        );
        assert_eq!(actual, expected);
    }

    #[test]
    fn global_trim_format_flag_row() {
        let row = make_record(vec!["a", "b"], vec!["    c ", " b c    d  e  "]);
        let expected = make_record(vec!["a", "b"], vec!["c", "b c d e"]);
        let closure_flags = ClosureFlags {
            format_flag: true,
            ..Default::default()
        };

        let actual = action(
            &row,
            Span::test_data(),
            None,
            &closure_flags,
            &trim,
            ActionMode::Global,
        );
        assert_eq!(actual, expected);
    }

    #[test]
    fn global_trim_format_flag_table() {
        let row = Value::List {
            vals: vec![
                Value::String {
                    val: "  a    b     c    d  ".to_string(),
                    span: Span::test_data(),
                },
                Value::Int {
                    val: 65,
                    span: Span::test_data(),
                },
                Value::String {
                    val: " b c  d e   f".to_string(),
                    span: Span::test_data(),
                },
            ],
            span: Span::test_data(),
        };
        let expected = Value::List {
            vals: vec![
                Value::String {
                    val: "a b c d".to_string(),
                    span: Span::test_data(),
                },
                Value::Int {
                    val: 65,
                    span: Span::test_data(),
                },
                Value::String {
                    val: "b c d e f".to_string(),
                    span: Span::test_data(),
                },
            ],
            span: Span::test_data(),
        };

        let closure_flags = ClosureFlags {
            format_flag: true,
            ..Default::default()
        };

        let actual = action(
            &row,
            Span::test_data(),
            None,
            &closure_flags,
            &trim,
            ActionMode::Global,
        );
        assert_eq!(actual, expected);
    }

    #[test]
    fn trims_custom_chars_format_flag() {
        let word = Value::test_string(".Value1.a..lot...of....dots.");
        let expected = Value::test_string("Value1.a.lot.of.dots");
        let closure_flags = ClosureFlags {
            format_flag: true,
            ..Default::default()
        };

        let actual = action(
            &word,
            Span::test_data(),
            Some('.'),
            &closure_flags,
            &trim,
            ActionMode::Local,
        );
        assert_eq!(actual, expected);
    }

    #[test]
    fn trims_all_format_flag_whitespace() {
        let word = Value::test_string(" nushell    is     great   ");
        let expected = Value::test_string("nushellisgreat");
        let closure_flags = ClosureFlags {
            format_flag: true,
            all_flag: true,
            ..Default::default()
        };

        let actual = action(
            &word,
            Span::test_data(),
            Some(' '),
            &closure_flags,
            &trim,
            ActionMode::Local,
        );
        assert_eq!(actual, expected);
    }

    #[test]
    fn trims_all_format_flag_global() {
        let word = Value::test_string(" nushell    is     great   ");
        let expected = Value::test_string("nushellisgreat");
        let closure_flags = ClosureFlags {
            format_flag: true,
            all_flag: true,
            ..Default::default()
        };

        let actual = action(
            &word,
            Span::test_data(),
            Some(' '),
            &closure_flags,
            &trim,
            ActionMode::Global,
        );
        assert_eq!(actual, expected);
    }
}
