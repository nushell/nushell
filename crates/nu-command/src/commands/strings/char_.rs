use crate::prelude::*;
use indexmap::indexmap;
use indexmap::map::IndexMap;
use lazy_static::lazy_static;
use nu_engine::{FromValue, WholeStreamCommand};
use nu_errors::ShellError;
use nu_protocol::{Signature, SyntaxShape, TaggedDictBuilder, UntaggedValue, Value};
use nu_source::Tagged;

// Character used to separate directories in a Path Environment variable on windows is ";"
#[cfg(target_family = "windows")]
const ENV_PATH_SEPARATOR_CHAR: char = ';';
// Character used to separate directories in a Path Environment variable on linux/mac/unix is ":"
#[cfg(not(target_family = "windows"))]
const ENV_PATH_SEPARATOR_CHAR: char = ':';

pub struct Char;

struct CharArgs {
    name: Option<Tagged<String>>,
    rest: Vec<Value>,
    list: bool,
    unicode: bool,
}

lazy_static! {
    static ref CHAR_MAP: IndexMap<&'static str, String> = indexmap! {
        // These are some regular characters that either can't be used or
        // it's just easier to use them like this.

        // This are the "normal" characters section
        "newline" => '\n'.to_string(),
        "enter" => '\n'.to_string(),
        "nl" => '\n'.to_string(),
        "tab" => '\t'.to_string(),
        "sp" => ' '.to_string(),
        "space" => ' '.to_string(),
        "pipe" => '|'.to_string(),
        "left_brace" => '{'.to_string(),
        "lbrace" => '{'.to_string(),
        "right_brace" => '}'.to_string(),
        "rbrace" => '}'.to_string(),
        "left_paren" => '('.to_string(),
        "lp" => '('.to_string(),
        "lparen" => '('.to_string(),
        "right_paren" => ')'.to_string(),
        "rp" => ')'.to_string(),
        "rparen" => ')'.to_string(),
        "left_bracket" => '['.to_string(),
        "lbracket" => '['.to_string(),
        "right_bracket" => ']'.to_string(),
        "rbracket" => ']'.to_string(),
        "single_quote" => '\''.to_string(),
        "squote" => '\''.to_string(),
        "sq" => '\''.to_string(),
        "double_quote" => '\"'.to_string(),
        "dquote" => '\"'.to_string(),
        "dq" => '\"'.to_string(),
        "path_sep" => std::path::MAIN_SEPARATOR.to_string(),
        "psep" => std::path::MAIN_SEPARATOR.to_string(),
        "separator" => std::path::MAIN_SEPARATOR.to_string(),
        "esep" => ENV_PATH_SEPARATOR_CHAR.to_string(),
        "env_sep" => ENV_PATH_SEPARATOR_CHAR.to_string(),
        "tilde" => '~'.to_string(),                                // ~
        "twiddle" => '~'.to_string(),                              // ~
        "squiggly" => '~'.to_string(),                             // ~
        "home" => '~'.to_string(),                                 // ~
        "hash" => '#'.to_string(),                                 // #
        "hashtag" => '#'.to_string(),                              // #
        "pound_sign" => '#'.to_string(),                           // #
        "sharp" => '#'.to_string(),                                // #
        "root" => '#'.to_string(),                                 // #
        "comma" => ','.to_string(), // , comma
        "semicolon" => ';'.to_string(), // ; semicolon
        "dollar" => '$'.to_string(), // $ dollar
        "at" => '@'.to_string(), // @ at
        "minus" => '-'.to_string(), // - minus
        "subtract" => '-'.to_string(),
        "dash" => '-'.to_string(),
        "plus" => '+'.to_string(), // + plus
        "add" => '+'.to_string(),
        "divide" => '/'.to_string(), // / divide, slash
        "slash" => '/'.to_string(),
        "backslash" => '\\'.to_string(),// \ backslash
        "percent" => '%'.to_string(), // % percent
        "multiply" => '*'.to_string(), // * multiply
        "greater_than" => '>'.to_string(), // > greater_than
        "gt" => '>'.to_string(),
        "less_than" => '<'.to_string(), // < less_than
        "lt" => '<'.to_string(),
        "and" => '&'.to_string(), // & and, ampersand
        "ampersand" => '&'.to_string(),
        "equal" => '='.to_string(), // = equal
        "eq" => '='.to_string(),
        "double_equal" => "==".to_string(), // == double_equal
        "fat_arrow" => "=>".to_string(), // => fat_arrow
        "right_arrow" => "->".to_string(), // -> right_arrow
        "left_arrow" => "<-".to_string(), // <- left_arrow
        "question" => '?'.to_string(), // ? question, q
        "q" => '?'.to_string(),
        "colon" => ':'.to_string(), // : colon
        "underscore" => '_'.to_string(),
        "u" => '_'.to_string(),

        // This is the unicode section
        // Unicode names came from https://www.compart.com/en/unicode
        // Private Use Area (U+E000-U+F8FF)
        // Unicode can't be mixed with Ansi or it will break width calculation
        "branch" => '\u{e0a0}'.to_string(),                        // 
        "segment" => '\u{e0b0}'.to_string(),                       // 

        "identical_to" => '\u{2261}'.to_string(),                  // ≡
        "hamburger" => '\u{2261}'.to_string(),                     // ≡
        "not_identical_to" => '\u{2262}'.to_string(),              // ≢
        "branch_untracked" => '\u{2262}'.to_string(),              // ≢
        "strictly_equivalent_to" => '\u{2263}'.to_string(),        // ≣
        "branch_identical" => '\u{2263}'.to_string(),              // ≣

        "upwards_arrow" => '\u{2191}'.to_string(),                 // ↑
        "branch_ahead" => '\u{2191}'.to_string(),                  // ↑
        "downwards_arrow" => '\u{2193}'.to_string(),               // ↓
        "branch_behind" => '\u{2193}'.to_string(),                 // ↓
        "up_down_arrow" => '\u{2195}'.to_string(),                 // ↕
        "branch_ahead_behind" => '\u{2195}'.to_string(),           // ↕

        "black_right_pointing_triangle" => '\u{25b6}'.to_string(), // ▶
        "prompt" => '\u{25b6}'.to_string(),                        // ▶
        "vector_or_cross_product" => '\u{2a2f}'.to_string(),       // ⨯
        "failed" => '\u{2a2f}'.to_string(),                        // ⨯
        "high_voltage_sign" => '\u{26a1}'.to_string(),             // ⚡
        "elevated" => '\u{26a1}'.to_string(),                      // ⚡

        // This is the emoji section
        // Weather symbols
        "sun" => "☀️".to_string(),
        "sunny" => "☀️".to_string(),
        "sunrise" => "☀️".to_string(),
        "moon" => "🌛".to_string(),
        "cloudy" => "☁️".to_string(),
        "cloud" => "☁️".to_string(),
        "clouds" => "☁️".to_string(),
        "rainy" => "🌦️".to_string(),
        "rain" => "🌦️".to_string(),
        "foggy" => "🌫️".to_string(),
        "fog" => "🌫️".to_string(),
        "mist" => '\u{2591}'.to_string(),
        "haze" => '\u{2591}'.to_string(),
        "snowy" => "❄️".to_string(),
        "snow" => "❄️".to_string(),
        "thunderstorm" => "🌩️".to_string(),
        "thunder" => "🌩️".to_string(),

        // This is the "other" section
        "bel" => '\x07'.to_string(),       // Terminal Bell
        "backspace" => '\x08'.to_string(), // Backspace
    };
}

impl WholeStreamCommand for Char {
    fn name(&self) -> &str {
        "char"
    }

    fn signature(&self) -> Signature {
        Signature::build("char")
            .optional(
                "character",
                SyntaxShape::Any,
                "the name of the character to output",
            )
            .rest("rest", SyntaxShape::String, "multiple Unicode bytes")
            .switch("list", "List all supported character names", Some('l'))
            .switch("unicode", "Unicode string i.e. 1f378", Some('u'))
    }

    fn usage(&self) -> &str {
        "Output special characters (e.g., 'newline')."
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
                example: r#"echo (char prompt) (char newline) (char hamburger)"#,
                result: Some(vec![
                    UntaggedValue::string("\u{25b6}").into(),
                    UntaggedValue::string("\n").into(),
                    UntaggedValue::string("\u{2261}").into(),
                ]),
            },
            Example {
                description: "Output Unicode character",
                example: r#"char -u 1f378"#,
                result: Some(vec![Value::from("\u{1f378}")]),
            },
            Example {
                description: "Output multi-byte Unicode character",
                example: r#"char -u 1F468 200D 1F466 200D 1F466"#,
                result: Some(vec![Value::from(
                    "\u{1F468}\u{200D}\u{1F466}\u{200D}\u{1F466}",
                )]),
            },
        ]
    }

    fn run(&self, args: CommandArgs) -> Result<OutputStream, ShellError> {
        let args_tag = args.call_info.name_tag.clone();
        let args = CharArgs {
            name: args.opt(0)?,
            rest: args.rest(1)?,
            list: args.has_flag("list"),
            unicode: args.has_flag("unicode"),
        };

        if args.list {
            Ok(CHAR_MAP
                .iter()
                .map(move |(name, s)| {
                    let mut dict = TaggedDictBuilder::with_capacity(&args_tag, 2);
                    dict.insert_untagged("name", UntaggedValue::string(*name));
                    dict.insert_untagged("character", UntaggedValue::string(s));
                    let unicode_parts: Vec<String> =
                        s.chars().map(|c| format!("{:x}", c as u32)).collect();
                    dict.insert_untagged("unicode", UntaggedValue::string(unicode_parts.join(" ")));
                    dict.into_value()
                })
                .into_output_stream())
        } else if let Some(name) = args.name {
            if args.unicode {
                if !args.rest.is_empty() {
                    // Setup a new buffer to put all the Unicode bytes in
                    let mut multi_byte = String::new();
                    // Get the first byte
                    let decoded_char = string_to_unicode_char(&name.item, &name.tag);
                    match decoded_char {
                        Ok(ch) => multi_byte.push(ch),
                        Err(e) => return Err(e),
                    }
                    // Get the rest of the bytes
                    for byte_part in args.rest {
                        let byte_part: Tagged<String> = FromValue::from_value(&byte_part)?;
                        let decoded_char = string_to_unicode_char(&byte_part, &byte_part.tag);
                        match decoded_char {
                            Ok(ch) => multi_byte.push(ch),
                            Err(e) => return Err(e),
                        }
                    }
                    Ok(OutputStream::one(
                        UntaggedValue::string(multi_byte).into_value(name.tag),
                    ))
                } else {
                    let decoded_char = string_to_unicode_char(&name.item, &name.tag);
                    if let Ok(ch) = decoded_char {
                        Ok(OutputStream::one(
                            UntaggedValue::string(ch).into_value(name.tag()),
                        ))
                    } else {
                        Err(ShellError::labeled_error(
                            "error decoding Unicode character",
                            "error decoding Unicode character",
                            name.tag(),
                        ))
                    }
                }
            } else {
                let special_character = str_to_character(&name.item);
                if let Some(output) = special_character {
                    Ok(OutputStream::one(
                        UntaggedValue::string(output).into_value(name.tag()),
                    ))
                } else {
                    Err(ShellError::labeled_error(
                        "error finding named character",
                        "error finding named character",
                        name.tag(),
                    ))
                }
            }
        } else {
            Err(ShellError::labeled_error(
                "char requires the name of the character",
                "missing name of the character",
                &args_tag,
            ))
        }
    }
}

fn string_to_unicode_char(s: &str, t: &Tag) -> Result<char, ShellError> {
    let decoded_char = u32::from_str_radix(s, 16)
        .ok()
        .and_then(std::char::from_u32);

    if let Some(ch) = decoded_char {
        Ok(ch)
    } else {
        Err(ShellError::labeled_error(
            "error decoding Unicode character",
            "error decoding Unicode character",
            t,
        ))
    }
}

fn str_to_character(s: &str) -> Option<String> {
    CHAR_MAP.get(s).map(|s| s.into())
}

#[cfg(test)]
mod tests {
    use super::Char;
    use super::ShellError;

    #[test]
    fn examples_work_as_expected() -> Result<(), ShellError> {
        use crate::examples::test as test_examples;

        test_examples(Char {})
    }
}
