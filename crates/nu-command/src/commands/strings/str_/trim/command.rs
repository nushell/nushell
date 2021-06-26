use super::operate;
use crate::commands::strings::str_::trim::ClosureFlags;
use crate::prelude::*;
use nu_engine::WholeStreamCommand;
use nu_errors::ShellError;
use nu_protocol::{Signature, SyntaxShape, Value};
pub struct SubCommand;

impl WholeStreamCommand for SubCommand {
    fn name(&self) -> &str {
        "str trim"
    }

    fn signature(&self) -> Signature {
        Signature::build("str trim")
            .rest(
                SyntaxShape::ColumnPath,
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
        "trims text"
    }

    fn run_with_actions(&self, args: CommandArgs) -> Result<ActionStream, ShellError> {
        operate(args, &trim)
    }

    fn examples(&self) -> Vec<Example> {
        vec![
            Example {
                description: "Trim whitespace",
                example: "echo 'Nu shell ' | str trim",
                result: Some(vec![Value::from("Nu shell")]),
            },
            Example {
                description: "Trim a specific character",
                example: "echo '=== Nu shell ===' | str trim -c '=' | str trim",
                result: Some(vec![Value::from("Nu shell")]),
            },
            Example {
                description: "Trim all characters",
                example: "echo ' Nu   shell ' | str trim -a",
                result: Some(vec![Value::from("Nushell")]),
            },
            Example {
                description: "Trim whitespace from the beginning of string",
                example: "echo ' Nu shell ' | str trim -l",
                result: Some(vec![Value::from("Nu shell ")]),
            },
            Example {
                description: "Trim a specific character",
                example: "echo '=== Nu shell ===' | str trim -c '='",
                result: Some(vec![Value::from(" Nu shell ")]),
            },
            Example {
                description: "Trim whitespace from the end of string",
                example: "echo ' Nu shell ' | str trim -r",
                result: Some(vec![Value::from(" Nu shell")]),
            },
            Example {
                description: "Trim a specific character",
                example: "echo '=== Nu shell ===' | str trim -r -c '='",
                result: Some(vec![Value::from("=== Nu shell ")]),
            },
        ]
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
    let delimiter = char_.unwrap_or(' ');
    let mut buf = vec![];
    let mut is_delim = false;
    let left_remove = left_trim | all_flag | both_flag | format_flag;
    let right_remove = right_trim | all_flag | both_flag | format_flag;
    let middle_remove = all_flag | format_flag;
    let middle_add = !all_flag && (*format_flag); // cases like -a -f
    for c in s.chars() {
        match c {
            x if x == delimiter && buf.is_empty() && left_remove => continue,
            x if x == delimiter => {
                is_delim = true;
                if !middle_remove {
                    buf.push(delimiter);
                }
            }
            _ => {
                if is_delim && middle_add {
                    buf.push(delimiter);
                    is_delim = false;
                }
                buf.push(c);
            }
        }
    }
    if right_remove {
        while buf.last() == Some(&delimiter) {
            buf.pop();
        }
    }
    buf.into_iter().collect()
}

#[cfg(test)]
mod tests {
    use super::ShellError;
    use super::SubCommand;
    use crate::commands::strings::str_::trim::command::trim;
    use crate::commands::strings::str_::trim::{action, ActionMode, ClosureFlags};
    use nu_protocol::row;
    use nu_source::Tag;
    use nu_test_support::value::{int, string, table};

    #[test]
    fn examples_work_as_expected() -> Result<(), ShellError> {
        use crate::examples::test as test_examples;
        test_examples(SubCommand {})
    }

    #[test]
    fn trims() {
        let word = string("andres ");
        let expected = string("andres");
        let closure_flags = ClosureFlags {
            both_flag: true,
            ..Default::default()
        };

        let actual = action(
            &word,
            Tag::unknown(),
            None,
            &closure_flags,
            &trim,
            ActionMode::Local,
        )
        .unwrap();
        assert_eq!(actual, expected);
    }

    #[test]
    fn trims_global() {
        let word = string(" global   ");
        let expected = string("global");
        let closure_flags = ClosureFlags {
            both_flag: true,
            ..Default::default()
        };

        let actual = action(
            &word,
            Tag::unknown(),
            None,
            &closure_flags,
            &trim,
            ActionMode::Global,
        )
        .unwrap();
        assert_eq!(actual, expected);
    }

    #[test]
    fn global_trim_ignores_numbers() {
        let number = int(2020);
        let expected = int(2020);
        let closure_flags = ClosureFlags {
            both_flag: true,
            ..Default::default()
        };

        let actual = action(
            &number,
            Tag::unknown(),
            None,
            &closure_flags,
            &trim,
            ActionMode::Global,
        )
        .unwrap();
        assert_eq!(actual, expected);
    }

    #[test]
    fn global_trim_row() {
        let row = row!["a".to_string() => string("    c "), " b ".to_string() => string("  d   ")];
        let expected = row!["a".to_string() => string("c"), " b ".to_string() => string("d")];
        let closure_flags = ClosureFlags {
            both_flag: true,
            ..Default::default()
        };

        let actual = action(
            &row,
            Tag::unknown(),
            None,
            &closure_flags,
            &trim,
            ActionMode::Global,
        )
        .unwrap();
        assert_eq!(actual, expected);
    }

    #[test]
    fn global_trim_table() {
        let row = table(&[string("  a  "), int(65), string(" d")]);
        let expected = table(&[string("a"), int(65), string("d")]);
        let closure_flags = ClosureFlags {
            both_flag: true,
            ..Default::default()
        };

        let actual = action(
            &row,
            Tag::unknown(),
            None,
            &closure_flags,
            &trim,
            ActionMode::Global,
        )
        .unwrap();
        assert_eq!(actual, expected);
    }

    #[test]
    fn trims_custom_character_both_ends() {
        let word = string("!#andres#!");
        let expected = string("#andres#");
        let closure_flags = ClosureFlags {
            both_flag: true,
            ..Default::default()
        };

        let actual = action(
            &word,
            Tag::unknown(),
            Some('!'),
            &closure_flags,
            &trim,
            ActionMode::Local,
        )
        .unwrap();
        assert_eq!(actual, expected);
    }
    #[test]
    fn trims_all_white_space() {
        let word = string(" Value1 a lot  of  spaces ");
        let expected = string("Value1alotofspaces");
        let closure_flags = ClosureFlags {
            all_flag: true,
            ..Default::default()
        };

        let actual = action(
            &word,
            Tag::unknown(),
            Some(' '),
            &closure_flags,
            &trim,
            ActionMode::Local,
        )
        .unwrap();
        assert_eq!(actual, expected);
    }

    #[test]
    fn global_trims_row_all_white_space() {
        let row = row!["a".to_string() => string("    nu    shell "), " b ".to_string() => string("  b c   d     e  ")];
        let expected =
            row!["a".to_string() => string("nushell"), " b ".to_string() => string("bcde")];
        let closure_flags = ClosureFlags {
            all_flag: true,
            ..Default::default()
        };

        let actual = action(
            &row,
            Tag::unknown(),
            None,
            &closure_flags,
            &trim,
            ActionMode::Global,
        )
        .unwrap();
        assert_eq!(actual, expected);
    }

    #[test]
    fn global_trims_table_all_white_space() {
        let row = table(&[string("  nu      shell   "), int(65), string(" d")]);
        let expected = table(&[string("nushell"), int(65), string("d")]);
        let closure_flags = ClosureFlags {
            all_flag: true,
            ..Default::default()
        };

        let actual = action(
            &row,
            Tag::unknown(),
            None,
            &closure_flags,
            &trim,
            ActionMode::Global,
        )
        .unwrap();
        assert_eq!(actual, expected);
    }

    #[test]
    fn trims_all_custom_character() {
        let word = string(".Value1.a.lot..of...dots.");
        let expected = string("Value1alotofdots");
        let closure_flags = ClosureFlags {
            all_flag: true,
            ..Default::default()
        };

        let actual = action(
            &word,
            Tag::unknown(),
            Some('.'),
            &closure_flags,
            &trim,
            ActionMode::Local,
        )
        .unwrap();
        assert_eq!(actual, expected);
    }

    #[test]
    fn global_trims_row_all_custom_character() {
        let row = row!["a".to_string() => string("!!!!nu!!shell!!!"), " b ".to_string() => string("!!b!c!!d!e!!")];
        let expected =
            row!["a".to_string() => string("nushell"), " b ".to_string() => string("bcde")];
        let closure_flags = ClosureFlags {
            all_flag: true,
            ..Default::default()
        };

        let actual = action(
            &row,
            Tag::unknown(),
            Some('!'),
            &closure_flags,
            &trim,
            ActionMode::Global,
        )
        .unwrap();
        assert_eq!(actual, expected);
    }

    #[test]
    fn global_trims_table_all_custom_character() {
        let row = table(&[string("##nu####shell##"), int(65), string("#d")]);
        let expected = table(&[string("nushell"), int(65), string("d")]);
        let closure_flags = ClosureFlags {
            all_flag: true,
            ..Default::default()
        };

        let actual = action(
            &row,
            Tag::unknown(),
            Some('#'),
            &closure_flags,
            &trim,
            ActionMode::Global,
        )
        .unwrap();
        assert_eq!(actual, expected);
    }
    #[test]
    fn trims_whitespace_from_left() {
        let word = string(" andres ");
        let expected = string("andres ");
        let closure_flags = ClosureFlags {
            left_trim: true,
            ..Default::default()
        };

        let actual = action(
            &word,
            Tag::unknown(),
            None,
            &closure_flags,
            &trim,
            ActionMode::Local,
        )
        .unwrap();
        assert_eq!(actual, expected);
    }

    #[test]
    fn global_trim_left_ignores_numbers() {
        let number = int(2020);
        let expected = int(2020);
        let closure_flags = ClosureFlags {
            left_trim: true,
            ..Default::default()
        };

        let actual = action(
            &number,
            Tag::unknown(),
            None,
            &closure_flags,
            &trim,
            ActionMode::Global,
        )
        .unwrap();
        assert_eq!(actual, expected);
    }

    #[test]
    fn trims_left_global() {
        let word = string(" global   ");
        let expected = string("global   ");
        let closure_flags = ClosureFlags {
            left_trim: true,
            ..Default::default()
        };

        let actual = action(
            &word,
            Tag::unknown(),
            None,
            &closure_flags,
            &trim,
            ActionMode::Global,
        )
        .unwrap();
        assert_eq!(actual, expected);
    }

    #[test]
    fn global_trim_left_row() {
        let row = row!["a".to_string() => string("    c "), " b ".to_string() => string("  d   ")];
        let expected = row!["a".to_string() => string("c "), " b ".to_string() => string("d   ")];
        let closure_flags = ClosureFlags {
            left_trim: true,
            ..Default::default()
        };

        let actual = action(
            &row,
            Tag::unknown(),
            None,
            &closure_flags,
            &trim,
            ActionMode::Global,
        )
        .unwrap();
        assert_eq!(actual, expected);
    }

    #[test]
    fn global_trim_left_table() {
        let row = table(&[string("  a  "), int(65), string(" d")]);
        let expected = table(&[string("a  "), int(65), string("d")]);
        let closure_flags = ClosureFlags {
            left_trim: true,
            ..Default::default()
        };

        let actual = action(
            &row,
            Tag::unknown(),
            None,
            &closure_flags,
            &trim,
            ActionMode::Global,
        )
        .unwrap();
        assert_eq!(actual, expected);
    }

    #[test]
    fn trims_custom_chars_from_left() {
        let word = string("!!! andres !!!");
        let expected = string(" andres !!!");
        let closure_flags = ClosureFlags {
            left_trim: true,
            ..Default::default()
        };

        let actual = action(
            &word,
            Tag::unknown(),
            Some('!'),
            &closure_flags,
            &trim,
            ActionMode::Local,
        )
        .unwrap();
        assert_eq!(actual, expected);
    }
    #[test]
    fn trims_whitespace_from_right() {
        let word = string(" andres ");
        let expected = string(" andres");
        let closure_flags = ClosureFlags {
            right_trim: true,
            ..Default::default()
        };

        let actual = action(
            &word,
            Tag::unknown(),
            None,
            &closure_flags,
            &trim,
            ActionMode::Local,
        )
        .unwrap();
        assert_eq!(actual, expected);
    }

    #[test]
    fn trims_right_global() {
        let word = string(" global   ");
        let expected = string(" global");
        let closure_flags = ClosureFlags {
            right_trim: true,
            ..Default::default()
        };

        let actual = action(
            &word,
            Tag::unknown(),
            None,
            &closure_flags,
            &trim,
            ActionMode::Global,
        )
        .unwrap();
        assert_eq!(actual, expected);
    }

    #[test]
    fn global_trim_right_ignores_numbers() {
        let number = int(2020);
        let expected = int(2020);
        let closure_flags = ClosureFlags {
            right_trim: true,
            ..Default::default()
        };

        let actual = action(
            &number,
            Tag::unknown(),
            None,
            &closure_flags,
            &trim,
            ActionMode::Global,
        )
        .unwrap();
        assert_eq!(actual, expected);
    }

    #[test]
    fn global_trim_right_row() {
        let row = row!["a".to_string() => string("    c "), " b ".to_string() => string("  d   ")];
        let expected = row!["a".to_string() => string("    c"), " b ".to_string() => string("  d")];
        let closure_flags = ClosureFlags {
            right_trim: true,
            ..Default::default()
        };

        let actual = action(
            &row,
            Tag::unknown(),
            None,
            &closure_flags,
            &trim,
            ActionMode::Global,
        )
        .unwrap();
        assert_eq!(actual, expected);
    }

    #[test]
    fn global_trim_right_table() {
        let row = table(&[string("  a  "), int(65), string(" d")]);
        let expected = table(&[string("  a"), int(65), string(" d")]);
        let closure_flags = ClosureFlags {
            right_trim: true,
            ..Default::default()
        };

        let actual = action(
            &row,
            Tag::unknown(),
            None,
            &closure_flags,
            &trim,
            ActionMode::Global,
        )
        .unwrap();
        assert_eq!(actual, expected);
    }

    #[test]
    fn trims_custom_chars_from_right() {
        let word = string("#@! andres !@#");
        let expected = string("#@! andres !@");
        let closure_flags = ClosureFlags {
            right_trim: true,
            ..Default::default()
        };

        let actual = action(
            &word,
            Tag::unknown(),
            Some('#'),
            &closure_flags,
            &trim,
            ActionMode::Local,
        )
        .unwrap();
        assert_eq!(actual, expected);
    }

    #[test]
    fn trims_whitespace_format_flag() {
        let word = string(" nushell    is     great ");
        let expected = string("nushell is great");
        let closure_flags = ClosureFlags {
            format_flag: true,
            ..Default::default()
        };

        let actual = action(
            &word,
            Tag::unknown(),
            Some(' '),
            &closure_flags,
            &trim,
            ActionMode::Local,
        )
        .unwrap();
        assert_eq!(actual, expected);
    }

    #[test]
    fn trims_format_flag_global() {
        let word = string("global ");
        let expected = string("global");
        let closure_flags = ClosureFlags {
            format_flag: true,
            ..Default::default()
        };

        let actual = action(
            &word,
            Tag::unknown(),
            Some(' '),
            &closure_flags,
            &trim,
            ActionMode::Local,
        )
        .unwrap();
        assert_eq!(actual, expected);
    }
    #[test]
    fn global_trim_format_flag_ignores_numbers() {
        let number = int(2020);
        let expected = int(2020);
        let closure_flags = ClosureFlags {
            format_flag: true,
            ..Default::default()
        };

        let actual = action(
            &number,
            Tag::unknown(),
            None,
            &closure_flags,
            &trim,
            ActionMode::Global,
        )
        .unwrap();
        assert_eq!(actual, expected);
    }

    #[test]
    fn global_trim_format_flag_row() {
        let row = row!["a".to_string() => string("    c "), " b ".to_string() => string(" b c    d  e  ")];
        let expected = row!["a".to_string() => string("c"), " b ".to_string() => string("b c d e")];
        let closure_flags = ClosureFlags {
            format_flag: true,
            ..Default::default()
        };

        let actual = action(
            &row,
            Tag::unknown(),
            None,
            &closure_flags,
            &trim,
            ActionMode::Global,
        )
        .unwrap();
        assert_eq!(actual, expected);
    }

    #[test]
    fn global_trim_format_flag_table() {
        let row = table(&[
            string("  a    b     c    d  "),
            int(65),
            string(" b c  d e   f"),
        ]);
        let expected = table(&[string("a b c d"), int(65), string("b c d e f")]);
        let closure_flags = ClosureFlags {
            format_flag: true,
            ..Default::default()
        };

        let actual = action(
            &row,
            Tag::unknown(),
            None,
            &closure_flags,
            &trim,
            ActionMode::Global,
        )
        .unwrap();
        assert_eq!(actual, expected);
    }

    #[test]
    fn trims_custom_chars_format_flag() {
        let word = string(".Value1.a..lot...of....dots.");
        let expected = string("Value1.a.lot.of.dots");
        let closure_flags = ClosureFlags {
            format_flag: true,
            ..Default::default()
        };

        let actual = action(
            &word,
            Tag::unknown(),
            Some('.'),
            &closure_flags,
            &trim,
            ActionMode::Local,
        )
        .unwrap();
        assert_eq!(actual, expected);
    }

    #[test]
    fn trims_all_format_flag_whitespace() {
        let word = string(" nushell    is     great   ");
        let expected = string("nushellisgreat");
        let closure_flags = ClosureFlags {
            format_flag: true,
            all_flag: true,
            ..Default::default()
        };

        let actual = action(
            &word,
            Tag::unknown(),
            Some(' '),
            &closure_flags,
            &trim,
            ActionMode::Local,
        )
        .unwrap();
        assert_eq!(actual, expected);
    }

    #[test]
    fn trims_all_format_flag_global() {
        let word = string(" nushell    is     great   ");
        let expected = string("nushellisgreat");
        let closure_flags = ClosureFlags {
            format_flag: true,
            all_flag: true,
            ..Default::default()
        };

        let actual = action(
            &word,
            Tag::unknown(),
            Some(' '),
            &closure_flags,
            &trim,
            ActionMode::Global,
        )
        .unwrap();
        assert_eq!(actual, expected);
    }
}
