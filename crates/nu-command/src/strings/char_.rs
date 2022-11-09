use indexmap::indexmap;
use indexmap::map::IndexMap;
use lazy_static::lazy_static;
use nu_engine::CallExt;
use nu_protocol::{
    ast::Call, engine::Command, Category, Example, IntoInterruptiblePipelineData, IntoPipelineData,
    PipelineData, ShellError, Signature, Span, SyntaxShape, Type, Value,
};

// Character used to separate directories in a Path Environment variable on windows is ";"
#[cfg(target_family = "windows")]
const ENV_PATH_SEPARATOR_CHAR: char = ';';
// Character used to separate directories in a Path Environment variable on linux/mac/unix is ":"
#[cfg(not(target_family = "windows"))]
const ENV_PATH_SEPARATOR_CHAR: char = ':';

#[derive(Clone)]
pub struct Char;

lazy_static! {
    static ref CHAR_MAP: IndexMap<&'static str, String> = indexmap! {
        // These are some regular characters that either can't be used or
        // it's just easier to use them like this.

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
    };
}

impl Command for Char {
    fn name(&self) -> &str {
        "char"
    }

    fn signature(&self) -> Signature {
        Signature::build("char")
            .input_output_types(vec![(Type::Nothing, Type::String)])
            .optional(
                "character",
                SyntaxShape::Any,
                "the name of the character to output",
            )
            .rest("rest", SyntaxShape::Any, "multiple Unicode bytes")
            .switch("list", "List all supported character names", Some('l'))
            .switch("unicode", "Unicode string i.e. 1f378", Some('u'))
            .switch("integer", "Create a codepoint from an integer", Some('i'))
            .category(Category::Strings)
    }

    fn usage(&self) -> &str {
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
                description: "Output prompt character, newline and a hamburger menu character",
                example: r#"(char prompt) + (char newline) + (char hamburger)"#,
                result: Some(Value::test_string("\u{25b6}\n\u{2261}")),
            },
            Example {
                description: "Output Unicode character",
                example: r#"char -u 1f378"#,
                result: Some(Value::test_string("\u{1f378}")),
            },
            Example {
                description: "Create Unicode from integer codepoint values",
                example: r#"char -i (0x60 + 1) (0x60 + 2)"#,
                result: Some(Value::test_string("ab")),
            },
            Example {
                description: "Output multi-byte Unicode character",
                example: r#"char -u 1F468 200D 1F466 200D 1F466"#,
                result: Some(Value::test_string(
                    "\u{1F468}\u{200D}\u{1F466}\u{200D}\u{1F466}",
                )),
            },
        ]
    }

    fn run(
        &self,
        engine_state: &nu_protocol::engine::EngineState,
        stack: &mut nu_protocol::engine::Stack,
        call: &Call,
        _input: PipelineData,
    ) -> Result<nu_protocol::PipelineData, ShellError> {
        let call_span = call.head;
        // handle -l flag
        if call.has_flag("list") {
            return Ok(CHAR_MAP
                .iter()
                .map(move |(name, s)| {
                    let cols = vec!["name".into(), "character".into(), "unicode".into()];
                    let name: Value = Value::string(String::from(*name), call_span);
                    let character = Value::string(s, call_span);
                    let unicode = Value::string(
                        s.chars()
                            .map(|c| format!("{:x}", c as u32))
                            .collect::<Vec<String>>()
                            .join(" "),
                        call_span,
                    );
                    let vals = vec![name, character, unicode];
                    Value::Record {
                        cols,
                        vals,
                        span: call_span,
                    }
                })
                .into_pipeline_data(engine_state.ctrlc.clone()));
        }
        // handle -u flag
        if call.has_flag("integer") {
            let args: Vec<i64> = call.rest(engine_state, stack, 0)?;
            if args.is_empty() {
                return Err(ShellError::MissingParameter(
                    "missing at least one unicode character".into(),
                    call_span,
                ));
            }
            let mut multi_byte = String::new();
            for (i, &arg) in args.iter().enumerate() {
                let span = call
                    .positional_nth(i)
                    .expect("Unexpected missing argument")
                    .span;
                multi_byte.push(integer_to_unicode_char(arg, &span)?)
            }
            Ok(Value::string(multi_byte, call_span).into_pipeline_data())
        } else if call.has_flag("unicode") {
            let args: Vec<String> = call.rest(engine_state, stack, 0)?;
            if args.is_empty() {
                return Err(ShellError::MissingParameter(
                    "missing at least one unicode character".into(),
                    call_span,
                ));
            }
            let mut multi_byte = String::new();
            for (i, arg) in args.iter().enumerate() {
                let span = call
                    .positional_nth(i)
                    .expect("Unexpected missing argument")
                    .span;
                multi_byte.push(string_to_unicode_char(arg, &span)?)
            }
            Ok(Value::string(multi_byte, call_span).into_pipeline_data())
        } else {
            let args: Vec<String> = call.rest(engine_state, stack, 0)?;
            if args.is_empty() {
                return Err(ShellError::MissingParameter(
                    "missing name of the character".into(),
                    call_span,
                ));
            }
            let special_character = str_to_character(&args[0]);
            if let Some(output) = special_character {
                Ok(Value::string(output, call_span).into_pipeline_data())
            } else {
                Err(ShellError::UnsupportedInput(
                    "error finding named character".into(),
                    call.positional_nth(0)
                        .expect("Unexpected missing argument")
                        .span,
                ))
            }
        }
    }
}

fn integer_to_unicode_char(value: i64, t: &Span) -> Result<char, ShellError> {
    let decoded_char = value.try_into().ok().and_then(std::char::from_u32);

    if let Some(ch) = decoded_char {
        Ok(ch)
    } else {
        Err(ShellError::UnsupportedInput(
            "not a valid Unicode codepoint".into(),
            *t,
        ))
    }
}

fn string_to_unicode_char(s: &str, t: &Span) -> Result<char, ShellError> {
    let decoded_char = u32::from_str_radix(s, 16)
        .ok()
        .and_then(std::char::from_u32);

    if let Some(ch) = decoded_char {
        Ok(ch)
    } else {
        Err(ShellError::UnsupportedInput(
            "error decoding Unicode character".into(),
            *t,
        ))
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
