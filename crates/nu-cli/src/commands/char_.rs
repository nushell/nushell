use crate::commands::WholeStreamCommand;
use crate::prelude::*;
use nu_errors::ShellError;
use nu_protocol::{ReturnSuccess, Signature, SyntaxShape, UntaggedValue, Value};
use nu_source::Tagged;

pub struct Char;

#[derive(Deserialize)]
struct CharArgs {
    name: Tagged<String>,
}

#[async_trait]
impl WholeStreamCommand for Char {
    fn name(&self) -> &str {
        "char"
    }

    fn signature(&self) -> Signature {
        Signature::build("ansi").required(
            "character",
            SyntaxShape::Any,
            "the name of the character to output",
        )
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
        ]
    }

    async fn run(
        &self,
        args: CommandArgs,
        registry: &CommandRegistry,
    ) -> Result<OutputStream, ShellError> {
        let (CharArgs { name }, _) = args.process(&registry).await?;

        let special_character = str_to_character(&name.item);

        if let Some(output) = special_character {
            Ok(OutputStream::one(ReturnSuccess::value(
                UntaggedValue::string(output).into_value(name.tag()),
            )))
        } else {
            Err(ShellError::labeled_error(
                "Unknown character",
                "unknown character",
                name.tag(),
            ))
        }
    }
}

fn str_to_character(s: &str) -> Option<String> {
    match s {
        "newline" | "enter" | "nl" => Some("\n".into()),
        "tab" => Some("\t".into()),
        "sp" | "space" => Some(" ".into()),
        // Unicode names came from https://www.compart.com/en/unicode
        // Private Use Area (U+E000-U+F8FF)
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
        "sun" => Some("\x1b[33;1m\u{2600}\x1b[0m".to_string()), // Yellow Bold ☀
        "moon" => Some("\x1b[36m\u{263d}\x1b[0m".to_string()),  // Cyan ☽
        "clouds" => Some("\x1b[37;1m\u{2601}\x1b[0m".to_string()), // White Bold ☁
        "rain" => Some("\x1b[37;1m\u{2614}\x1b[0m".to_string()), // White Bold ☔
        "fog" => Some("\x1b[37;1m\u{2592}\x1b[0m".to_string()), // White Bold ▒
        "mist" => Some("\x1b[34m\u{2591}\x1b[0m".to_string()),  // Blue ░
        "haze" => Some("\x1b[33m\u{2591}\x1b[0m".to_string()),  // Yellow ░
        "snow" => Some("\x1b[37;1m\u{2744}\x1b[0m".to_string()), // White Bold ❄
        "thunderstorm" => Some("\x1b[33;1m\u{26a1}\x1b[0m".to_string()), // Yellow Bold ⚡
        _ => None,
    }
}

// sun=$(get_config "sun" || echo "\033[33;1m\xe2\x98\x80")
// moon=$(get_config "moon" || echo "\033[36m\xe2\x98\xbd")
// clouds=$(get_config "clouds" || echo "\033[37;1m\xe2\x98\x81")
// rain=$(get_config "rain" || echo "\033[37;1m\xe2\x98\x94")
// fog=$(get_config "fog" || echo "\033[37;1m\xe2\x96\x92")
// mist=$(get_config "mist" || echo "\033[34m\xe2\x96\x91")
// haze=$(get_config "haze" || echo "\033[33m\xe2\x96\x91")
// snow=$(get_config "snow" || echo "\033[37;1m\xe2\x9d\x84")
// thunderstorm=$(get_config "thunderstorm" || echo "\033[33;1m\xe2\x9a\xa1")

#[cfg(test)]
mod tests {
    use super::Char;

    #[test]
    fn examples_work_as_expected() {
        use crate::examples::test as test_examples;

        test_examples(Char {})
    }
}
