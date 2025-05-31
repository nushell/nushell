use indexmap::{IndexMap, indexmap};
use nu_engine::command_prelude::*;

use nu_protocol::Signals;
use std::collections::HashSet;
use std::sync::LazyLock;

// Character used to separate directories in a Path Environment variable on windows is ";"
#[cfg(target_family = "windows")]
const ENV_PATH_SEPARATOR_CHAR: char = ';';
// Character used to separate directories in a Path Environment variable on linux/mac/unix is ":"
#[cfg(not(target_family = "windows"))]
const ENV_PATH_SEPARATOR_CHAR: char = ':';

// Character used to separate directories in a Path Environment variable on windows is ";"
#[cfg(target_family = "windows")]
const LINE_SEPARATOR_CHAR: &str = "\r\n";
// Character used to separate directories in a Path Environment variable on linux/mac/unix is ":"
#[cfg(not(target_family = "windows"))]
const LINE_SEPARATOR_CHAR: char = '\n';

#[derive(Clone)]
pub struct Char;

static CHAR_MAP: LazyLock<IndexMap<&'static str, String>> = LazyLock::new(|| {
    indexmap! {
        // These are some regular characters that either can't be used or
        // it's just easier to use them like this.

        "nul" => '\x00'.to_string(),                                // nul character, 0x00
        "null_byte" => '\x00'.to_string(),                          // nul character, 0x00
        "zero_byte" => '\x00'.to_string(),                          // nul character, 0x00
        // This are the "normal" characters section
        "newline" => '\n'.to_string(),
        "enter" => '\n'.to_string(),
        "nl" => '\n'.to_string(),
        "line_feed" => '\n'.to_string(),
        "lf" => '\n'.to_string(),
        "carriage_return" => '\r'.to_string(),
        "cr" => '\r'.to_string(),
        "crlf" => "\r\n".to_string(),
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
        "rparen" => ')'.to_string(),
        "rp" => ')'.to_string(),
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
        "eol" => LINE_SEPARATOR_CHAR.to_string(),
        "lsep" => LINE_SEPARATOR_CHAR.to_string(),
        "line_sep" => LINE_SEPARATOR_CHAR.to_string(),
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

        // This is the unicode section
        // Unicode names came from https://www.compart.com/en/unicode
        // Private Use Area (U+E000-U+F8FF)
        // Unicode can't be mixed with Ansi or it will break width calculation
        "nf_branch" => '\u{e0a0}'.to_string(),                     // 
        "nf_segment" => '\u{e0b0}'.to_string(),                    // 
        "nf_left_segment" => '\u{e0b0}'.to_string(),               // 
        "nf_left_segment_thin" => '\u{e0b1}'.to_string(),          // 
        "nf_right_segment" => '\u{e0b2}'.to_string(),              // 
        "nf_right_segment_thin" => '\u{e0b3}'.to_string(),         // 
        "nf_git" => '\u{f1d3}'.to_string(),                        // 
        "nf_git_branch" => "\u{e709}\u{e0a0}".to_string(),         // 
        "nf_folder1" => '\u{f07c}'.to_string(),                    // 
        "nf_folder2" => '\u{f115}'.to_string(),                    // 
        "nf_house1" => '\u{f015}'.to_string(),                     // 
        "nf_house2" => '\u{f7db}'.to_string(),                     // 

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
        // https://www.babelstone.co.uk/Unicode/whatisit.html
        "sun" => "☀️".to_string(),         //2600 + fe0f
        "sunny" => "☀️".to_string(),       //2600 + fe0f
        "sunrise" => "☀️".to_string(),     //2600 + fe0f
        "moon" => "🌛".to_string(),        //1f31b
        "cloudy" => "☁️".to_string(),      //2601 + fe0f
        "cloud" => "☁️".to_string(),       //2601 + fe0f
        "clouds" => "☁️".to_string(),      //2601 + fe0f
        "rainy" => "🌦️".to_string(),       //1f326 + fe0f
        "rain" => "🌦️".to_string(),        //1f326 + fe0f
        "foggy" => "🌫️".to_string(),       //1f32b + fe0f
        "fog" => "🌫️".to_string(),         //1f32b + fe0f
        "mist" => '\u{2591}'.to_string(),  //2591
        "haze" => '\u{2591}'.to_string(),  //2591
        "snowy" => "❄️".to_string(),       //2744 + fe0f
        "snow" => "❄️".to_string(),        //2744 + fe0f
        "thunderstorm" => "🌩️".to_string(),//1f329 + fe0f
        "thunder" => "🌩️".to_string(),     //1f329 + fe0f

        // This is the "other" section
        "bel" => '\x07'.to_string(),       // Terminal Bell
        "backspace" => '\x08'.to_string(), // Backspace

        // separators
        "file_separator" => '\x1c'.to_string(),
        "file_sep"  => '\x1c'.to_string(),
        "fs" => '\x1c'.to_string(),
        "group_separator" => '\x1d'.to_string(),
        "group_sep" => '\x1d'.to_string(),
        "gs" => '\x1d'.to_string(),
        "record_separator" => '\x1e'.to_string(),
        "record_sep" => '\x1e'.to_string(),
        "rs" => '\x1e'.to_string(),
        "unit_separator" => '\x1f'.to_string(),
        "unit_sep" => '\x1f'.to_string(),
        "us" => '\x1f'.to_string(),
    }
});

static NO_OUTPUT_CHARS: LazyLock<HashSet<&'static str>> = LazyLock::new(|| {
    [
        // If the character is in the this set, we don't output it to prevent
        // the broken of `char --list` command table format and alignment.
        "nul",
        "null_byte",
        "zero_byte",
        "newline",
        "enter",
        "nl",
        "line_feed",
        "lf",
        "cr",
        "crlf",
        "bel",
        "backspace",
        "lsep",
        "line_sep",
        "eol",
    ]
    .into_iter()
    .collect()
});

impl Command for Char {
    fn name(&self) -> &str {
        "char"
    }

    fn signature(&self) -> Signature {
        Signature::build("char")
            .input_output_types(vec![(Type::Nothing, Type::Any)])
            .optional(
                "character",
                SyntaxShape::Any,
                "The name of the character to output.",
            )
            .rest("rest", SyntaxShape::Any, "Multiple Unicode bytes.")
            .switch("list", "List all supported character names", Some('l'))
            .switch("unicode", "Unicode string i.e. 1f378", Some('u'))
            .switch("integer", "Create a codepoint from an integer", Some('i'))
            .allow_variants_without_examples(true)
            .category(Category::Strings)
    }

    fn is_const(&self) -> bool {
        true
    }

    fn description(&self) -> &str {
        "Output special characters (e.g., 'newline')."
    }

    fn search_terms(&self) -> Vec<&str> {
        vec!["line break", "newline", "Unicode"]
    }

    fn examples(&self) -> Vec<Example> {
        vec![
            Example {
                description: "Output newline",
                example: r#"char newline"#,
                result: Some(Value::test_string("\n")),
            },
            Example {
                description: "List available characters",
                example: r#"char --list"#,
                result: None,
            },
            Example {
                description: "Output prompt character, newline and a hamburger menu character",
                example: r#"(char prompt) + (char newline) + (char hamburger)"#,
                result: Some(Value::test_string("\u{25b6}\n\u{2261}")),
            },
            Example {
                description: "Output Unicode character",
                example: r#"char --unicode 1f378"#,
                result: Some(Value::test_string("\u{1f378}")),
            },
            Example {
                description: "Create Unicode from integer codepoint values",
                example: r#"char --integer (0x60 + 1) (0x60 + 2)"#,
                result: Some(Value::test_string("ab")),
            },
            Example {
                description: "Output multi-byte Unicode character",
                example: r#"char --unicode 1F468 200D 1F466 200D 1F466"#,
                result: Some(Value::test_string(
                    "\u{1F468}\u{200D}\u{1F466}\u{200D}\u{1F466}",
                )),
            },
        ]
    }

    fn run_const(
        &self,
        working_set: &StateWorkingSet,
        call: &Call,
        _input: PipelineData,
    ) -> Result<PipelineData, ShellError> {
        let call_span = call.head;
        let list = call.has_flag_const(working_set, "list")?;
        let integer = call.has_flag_const(working_set, "integer")?;
        let unicode = call.has_flag_const(working_set, "unicode")?;

        // handle -l flag
        if list {
            return Ok(generate_character_list(
                working_set.permanent().signals().clone(),
                call.head,
            ));
        }

        // handle -i flag
        if integer {
            let int_args = call.rest_const(working_set, 0)?;
            handle_integer_flag(int_args, call_span)
        }
        // handle -u flag
        else if unicode {
            let string_args = call.rest_const(working_set, 0)?;
            handle_unicode_flag(string_args, call_span)
        }
        // handle the rest
        else {
            let string_args = call.rest_const(working_set, 0)?;
            handle_the_rest(string_args, call_span)
        }
    }

    fn run(
        &self,
        engine_state: &EngineState,
        stack: &mut Stack,
        call: &Call,
        _input: PipelineData,
    ) -> Result<PipelineData, ShellError> {
        let call_span = call.head;
        let list = call.has_flag(engine_state, stack, "list")?;
        let integer = call.has_flag(engine_state, stack, "integer")?;
        let unicode = call.has_flag(engine_state, stack, "unicode")?;

        // handle -l flag
        if list {
            return Ok(generate_character_list(
                engine_state.signals().clone(),
                call_span,
            ));
        }

        // handle -i flag
        if integer {
            let int_args = call.rest(engine_state, stack, 0)?;
            handle_integer_flag(int_args, call_span)
        }
        // handle -u flag
        else if unicode {
            let string_args = call.rest(engine_state, stack, 0)?;
            handle_unicode_flag(string_args, call_span)
        }
        // handle the rest
        else {
            let string_args = call.rest(engine_state, stack, 0)?;
            handle_the_rest(string_args, call_span)
        }
    }
}

fn generate_character_list(signals: Signals, call_span: Span) -> PipelineData {
    CHAR_MAP
        .iter()
        .map(move |(name, s)| {
            let character = if NO_OUTPUT_CHARS.contains(name) {
                Value::string("", call_span)
            } else {
                Value::string(s, call_span)
            };
            let unicode = Value::string(
                s.chars()
                    .map(|c| format!("{:x}", c as u32))
                    .collect::<Vec<String>>()
                    .join(" "),
                call_span,
            );
            let record = record! {
                "name" => Value::string(*name, call_span),
                "character" => character,
                "unicode" => unicode,
            };

            Value::record(record, call_span)
        })
        .into_pipeline_data(call_span, signals)
}

fn handle_integer_flag(
    int_args: Vec<Spanned<i64>>,
    call_span: Span,
) -> Result<PipelineData, ShellError> {
    if int_args.is_empty() {
        return Err(ShellError::MissingParameter {
            param_name: "missing at least one unicode character".into(),
            span: call_span,
        });
    }

    let str = int_args
        .into_iter()
        .map(integer_to_unicode_char)
        .collect::<Result<String, _>>()?;

    Ok(Value::string(str, call_span).into_pipeline_data())
}

fn handle_unicode_flag(
    string_args: Vec<Spanned<String>>,
    call_span: Span,
) -> Result<PipelineData, ShellError> {
    if string_args.is_empty() {
        return Err(ShellError::MissingParameter {
            param_name: "missing at least one unicode character".into(),
            span: call_span,
        });
    }

    let str = string_args
        .into_iter()
        .map(string_to_unicode_char)
        .collect::<Result<String, _>>()?;

    Ok(Value::string(str, call_span).into_pipeline_data())
}

fn handle_the_rest(
    string_args: Vec<Spanned<String>>,
    call_span: Span,
) -> Result<PipelineData, ShellError> {
    let Some(s) = string_args.first() else {
        return Err(ShellError::MissingParameter {
            param_name: "missing name of the character".into(),
            span: call_span,
        });
    };

    let special_character = str_to_character(&s.item);

    if let Some(output) = special_character {
        Ok(Value::string(output, call_span).into_pipeline_data())
    } else {
        Err(ShellError::TypeMismatch {
            err_message: "error finding named character".into(),
            span: s.span,
        })
    }
}

fn integer_to_unicode_char(value: Spanned<i64>) -> Result<char, ShellError> {
    let decoded_char = value.item.try_into().ok().and_then(std::char::from_u32);

    if let Some(ch) = decoded_char {
        Ok(ch)
    } else {
        Err(ShellError::TypeMismatch {
            err_message: "not a valid Unicode codepoint".into(),
            span: value.span,
        })
    }
}

fn string_to_unicode_char(s: Spanned<String>) -> Result<char, ShellError> {
    let decoded_char = u32::from_str_radix(&s.item, 16)
        .ok()
        .and_then(std::char::from_u32);

    if let Some(ch) = decoded_char {
        Ok(ch)
    } else {
        Err(ShellError::TypeMismatch {
            err_message: "error decoding Unicode character".into(),
            span: s.span,
        })
    }
}

fn str_to_character(s: &str) -> Option<String> {
    CHAR_MAP.get(s).map(|s| s.into())
}

#[cfg(test)]
mod tests {
    use super::Char;

    #[test]
    fn examples_work_as_expected() {
        use crate::test_examples;

        test_examples(Char {})
    }
}
