use crate::prelude::*;
use nu_engine::WholeStreamCommand;
use nu_errors::ShellError;
use nu_protocol::{ReturnSuccess, Signature, SyntaxShape, UntaggedValue, Value};
use nu_source::Tagged;

pub struct Char;

#[derive(Deserialize)]
struct CharArgs {
    name: Tagged<String>,
    unicode: bool,
}

#[async_trait]
impl WholeStreamCommand for Char {
    fn name(&self) -> &str {
        "char"
    }

    fn signature(&self) -> Signature {
        Signature::build("ansi")
            .required(
                "character",
                SyntaxShape::Any,
                "the name of the character to output",
            )
            .switch("unicode", "unicode string i.e. 1f378", Some('u'))
    }

    fn usage(&self) -> &str {
        "Output special characters (eg. 'newline')"
    }

    fn examples(&self) -> Vec<Example> {
        vec![
            Example {
                description: "Output newline",
                example: r#"char newline"#,
                result: Some(vec![Value::from("\n")]),
            },
            Example {
                description: "Output prompt character, newline and a hamburger character",
                example: r#"echo $(char prompt) $(char newline) $(char hamburger)"#,
                result: Some(vec![
                    UntaggedValue::string("\u{25b6}").into(),
                    UntaggedValue::string("\n").into(),
                    UntaggedValue::string("\u{2261}").into(),
                ]),
            },
            Example {
                description: "Output unicode character",
                example: r#"char -u 1f378"#,
                result: Some(vec![Value::from("\u{1f378}")]),
            },
        ]
    }

    async fn run(&self, args: CommandArgs) -> Result<OutputStream, ShellError> {
        let (CharArgs { name, unicode }, _) = args.process().await?;

        if unicode {
            let decoded_char = string_to_unicode_char(&name.item);
            if let Some(output) = decoded_char {
                Ok(OutputStream::one(ReturnSuccess::value(
                    UntaggedValue::string(output).into_value(name.tag()),
                )))
            } else {
                Err(ShellError::labeled_error(
                    "error decoding unicode character",
                    "error decoding unicode character",
                    name.tag(),
                ))
            }
        } else {
            let special_character = str_to_character(&name.item);
            if let Some(output) = special_character {
                Ok(OutputStream::one(ReturnSuccess::value(
                    UntaggedValue::string(output).into_value(name.tag()),
                )))
            } else {
                Err(ShellError::labeled_error(
                    "error finding named character",
                    "error finding named character",
                    name.tag(),
                ))
            }
        }
    }
}

fn string_to_unicode_char(s: &str) -> Option<char> {
    u32::from_str_radix(s, 16)
        .ok()
        .and_then(std::char::from_u32)
}

fn str_to_character(s: &str) -> Option<String> {
    match s {
        "newline" | "enter" | "nl" => Some("\n".into()),
        "tab" => Some("\t".into()),
        "sp" | "space" => Some(" ".into()),
        // Unicode names came from https://www.compart.com/en/unicode
        // Private Use Area (U+E000-U+F8FF)
        // Unicode can't be mixed with Ansi or it will break width calculation
        "branch" => Some('\u{e0a0}'.to_string()),  // 
        "segment" => Some('\u{e0b0}'.to_string()), // 

        "identical_to" | "hamburger" => Some('\u{2261}'.to_string()), // ≡
        "not_identical_to" | "branch_untracked" => Some('\u{2262}'.to_string()), // ≢
        "strictly_equivalent_to" | "branch_identical" => Some('\u{2263}'.to_string()), // ≣

        "upwards_arrow" | "branch_ahead" => Some('\u{2191}'.to_string()), // ↑
        "downwards_arrow" | "branch_behind" => Some('\u{2193}'.to_string()), // ↓
        "up_down_arrow" | "branch_ahead_behind" => Some('\u{2195}'.to_string()), // ↕

        "black_right_pointing_triangle" | "prompt" => Some('\u{25b6}'.to_string()), // ▶
        "vector_or_cross_product" | "failed" => Some('\u{2a2f}'.to_string()),       // ⨯
        "high_voltage_sign" | "elevated" => Some('\u{26a1}'.to_string()),           // ⚡
        "tilde" | "twiddle" | "squiggly" | "home" => Some("~".into()),              // ~
        "hash" | "hashtag" | "pound_sign" | "sharp" | "root" => Some("#".into()),   // #

        // Weather symbols
        "sun" | "sunny" | "sunrise" => Some("☀️".to_string()),
        "moon" => Some("🌛".to_string()),
        "cloudy" | "cloud" | "clouds" => Some("☁️".to_string()),
        "rainy" | "rain" => Some("🌦️".to_string()),
        "foggy" | "fog" => Some("🌫️".to_string()),
        "mist" | "haze" => Some("\u{2591}".to_string()),
        "snowy" | "snow" => Some("❄️".to_string()),
        "thunderstorm" | "thunder" => Some("🌩️".to_string()),

        // Reference for ansi codes https://gist.github.com/fnky/458719343aabd01cfb17a3a4f7296797
        // Another good reference http://ascii-table.com/ansi-escape-sequences.php

        // For setting title like `echo [$(char title) $(pwd) $(char bel)] | str collect`
        "title" => Some("\x1b]2;".to_string()), // ESC]2; xterm sets window title
        "bel" => Some('\x07'.to_string()),      // Terminal Bell
        "backspace" => Some('\x08'.to_string()), // Backspace

        // Ansi Erase Sequences
        "clear_screen" => Some("\x1b[J".to_string()), // clears the screen
        "clear_screen_from_cursor_to_end" => Some("\x1b[0J".to_string()), // clears from cursor until end of screen
        "clear_screen_from_cursor_to_beginning" => Some("\x1b[1J".to_string()), // clears from cursor to beginning of screen
        "cls" | "clear_entire_screen" => Some("\x1b[2J".to_string()), // clears the entire screen
        "erase_line" => Some("\x1b[K".to_string()),                   // clears the current line
        "erase_line_from_cursor_to_end" => Some("\x1b[0K".to_string()), // clears from cursor to end of line
        "erase_line_from_cursor_to_beginning" => Some("\x1b[1K".to_string()), // clears from cursor to start of line
        "erase_entire_line" => Some("\x1b[2K".to_string()),                   // clears entire line

        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::Char;
    use super::ShellError;

    #[test]
    fn examples_work_as_expected() -> Result<(), ShellError> {
        use crate::examples::test as test_examples;

        Ok(test_examples(Char {})?)
    }
}
