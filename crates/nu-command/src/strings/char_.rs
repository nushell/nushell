use indexmap::IndexMap;
use nu_engine::command_prelude::*;
use nu_protocol::{Parameter, Signals};
use nu_utils::consts::{ENV_PATH_SEPARATOR_CHAR, LINE_SEPARATOR_STR};
use std::path::MAIN_SEPARATOR;
use std::sync::LazyLock;

#[derive(Clone)]
pub struct Char;

/// for each character, a name and an optional list of aliases.
struct CharGroup {
    /// the actual character.
    ch: String,
    /// a candidate name for the character.
    name: &'static str,
    /// optional aliases for the character name.
    aliases: Vec<&'static str>,
}

/// some groups may resolve to the same character, because some
/// of these are platform-dependant.
static CHAR_GROUPS: LazyLock<Vec<CharGroup>> = LazyLock::new(|| {
    vec![
        // These are some regular characters that either can't be used or
        // it's just easier to use them like this.
        CharGroup {
            ch: '\x00'.to_string(),
            name: "nul",
            aliases: vec!["null_byte", "zero_byte"],
        },
        // This are the "normal" characters section
        CharGroup {
            ch: '\n'.to_string(),
            name: "newline",
            aliases: vec!["enter", "nl", "line_feed", "lf"],
        },
        CharGroup {
            ch: '\r'.to_string(),
            name: "carriage_return",
            aliases: vec!["cr"],
        },
        CharGroup {
            ch: "\r\n".to_string(),
            name: "crlf",
            aliases: vec![],
        },
        CharGroup {
            ch: '\t'.to_string(),
            name: "tab",
            aliases: vec![],
        },
        CharGroup {
            ch: ' '.to_string(),
            name: "sp",
            aliases: vec!["space"],
        },
        CharGroup {
            ch: '|'.to_string(),
            name: "pipe",
            aliases: vec![],
        },
        CharGroup {
            ch: '{'.to_string(),
            name: "left_brace",
            aliases: vec!["lbrace"],
        },
        CharGroup {
            ch: '}'.to_string(),
            name: "right_brace",
            aliases: vec!["rbrace"],
        },
        CharGroup {
            ch: '('.to_string(),
            name: "left_paren",
            aliases: vec!["lp", "lparen"],
        },
        CharGroup {
            ch: ')'.to_string(),
            name: "right_paren",
            aliases: vec!["rparen", "rp"],
        },
        CharGroup {
            ch: '['.to_string(),
            name: "left_bracket",
            aliases: vec!["lbracket"],
        },
        CharGroup {
            ch: ']'.to_string(),
            name: "right_bracket",
            aliases: vec!["rbracket"],
        },
        CharGroup {
            ch: '\''.to_string(),
            name: "single_quote",
            aliases: vec!["squote", "sq"],
        },
        CharGroup {
            ch: '\"'.to_string(),
            name: "double_quote",
            aliases: vec!["dquote", "dq"],
        },
        CharGroup {
            ch: '/'.to_string(),
            name: "forward_slash",
            aliases: vec!["slash", "fslash"],
        },
        CharGroup {
            ch: '\\'.to_string(),
            name: "back_slash",
            aliases: vec!["bslash"],
        },
        CharGroup {
            ch: MAIN_SEPARATOR.to_string(),
            name: "path_sep",
            aliases: vec!["psep", "separator"],
        },
        CharGroup {
            ch: LINE_SEPARATOR_STR.to_string(),
            name: "eol",
            aliases: vec!["lsep", "line_sep"],
        },
        CharGroup {
            ch: ENV_PATH_SEPARATOR_CHAR.to_string(),
            name: "esep",
            aliases: vec!["env_sep"],
        },
        CharGroup {
            ch: '~'.to_string(),
            name: "tilde",
            aliases: vec!["twiddle", "squiggly", "home"],
        },
        CharGroup {
            ch: '#'.to_string(),
            name: "hash",
            aliases: vec!["hashtag", "pound_sign", "sharp", "root"],
        },
        // This is the unicode section
        // Unicode names came from https://www.compart.com/en/unicode
        // Private Use Area (U+E000-U+F8FF)
        // Unicode can't be mixed with Ansi or it will break width calculation
        CharGroup {
            ch: '\u{e0a0}'.to_string(),
            name: "nf_branch",
            aliases: vec![],
        },
        CharGroup {
            ch: '\u{e0b0}'.to_string(),
            name: "nf_segment",
            aliases: vec!["nf_left_segment"],
        },
        CharGroup {
            ch: '\u{e0b1}'.to_string(),
            name: "nf_left_segment_thin",
            aliases: vec![],
        },
        CharGroup {
            ch: '\u{e0b2}'.to_string(),
            name: "nf_right_segment",
            aliases: vec![],
        },
        CharGroup {
            ch: '\u{e0b3}'.to_string(),
            name: "nf_right_segment_thin",
            aliases: vec![],
        },
        CharGroup {
            ch: '\u{f1d3}'.to_string(),
            name: "nf_git",
            aliases: vec![],
        },
        CharGroup {
            ch: "\u{e709}\u{e0a0}".to_string(),
            name: "nf_git_branch",
            aliases: vec![],
        },
        CharGroup {
            ch: '\u{f07c}'.to_string(),
            name: "nf_folder1",
            aliases: vec![],
        },
        CharGroup {
            ch: '\u{f115}'.to_string(),
            name: "nf_folder2",
            aliases: vec![],
        },
        CharGroup {
            ch: '\u{f015}'.to_string(),
            name: "nf_house1",
            aliases: vec![],
        },
        CharGroup {
            ch: '\u{f7db}'.to_string(),
            name: "nf_house2",
            aliases: vec![],
        },
        CharGroup {
            ch: '\u{2261}'.to_string(),
            name: "identical_to",
            aliases: vec!["hamburger"],
        },
        CharGroup {
            ch: '\u{2262}'.to_string(),
            name: "not_identical_to",
            aliases: vec!["branch_untracked"],
        },
        CharGroup {
            ch: '\u{2263}'.to_string(),
            name: "strictly_equivalent_to",
            aliases: vec!["branch_identical"],
        },
        CharGroup {
            ch: '\u{2191}'.to_string(),
            name: "upwards_arrow",
            aliases: vec!["branch_ahead"],
        },
        CharGroup {
            ch: '\u{2193}'.to_string(),
            name: "downwards_arrow",
            aliases: vec!["branch_behind"],
        },
        CharGroup {
            ch: '\u{2195}'.to_string(),
            name: "up_down_arrow",
            aliases: vec!["branch_ahead_behind"],
        },
        CharGroup {
            ch: '\u{25b6}'.to_string(),
            name: "black_right_pointing_triangle",
            aliases: vec!["prompt"],
        },
        CharGroup {
            ch: '\u{2a2f}'.to_string(),
            name: "vector_or_cross_product",
            aliases: vec!["failed"],
        },
        CharGroup {
            ch: '\u{26a1}'.to_string(),
            name: "high_voltage_sign",
            aliases: vec!["elevated"],
        },
        // This is the emoji section
        // Weather symbols
        // https://www.babelstone.co.uk/Unicode/whatisit.html
        CharGroup {
            ch: "☀️".to_string(),
            name: "sun",
            aliases: vec!["sunny", "sunrise"],
        },
        CharGroup {
            ch: "🌛".to_string(),
            name: "moon",
            aliases: vec![],
        },
        CharGroup {
            ch: "☁️".to_string(),
            name: "cloudy",
            aliases: vec!["cloud", "clouds"],
        },
        CharGroup {
            ch: "🌦️".to_string(),
            name: "rainy",
            aliases: vec!["rain"],
        },
        CharGroup {
            ch: "🌫️".to_string(),
            name: "foggy",
            aliases: vec!["fog"],
        },
        CharGroup {
            ch: '\u{2591}'.to_string(),
            name: "mist",
            aliases: vec!["haze"],
        },
        CharGroup {
            ch: "❄️".to_string(),
            name: "snowy",
            aliases: vec!["snow"],
        },
        CharGroup {
            ch: "🌩️".to_string(),
            name: "thunderstorm",
            aliases: vec!["thunder"],
        },
        // This is the "other" section
        CharGroup {
            ch: '\x07'.to_string(),
            name: "bel",
            aliases: vec![],
        },
        CharGroup {
            ch: '\x08'.to_string(),
            name: "backspace",
            aliases: vec![],
        },
        // separators
        CharGroup {
            ch: '\x1c'.to_string(),
            name: "file_separator",
            aliases: vec!["file_sep", "fs"],
        },
        CharGroup {
            ch: '\x1d'.to_string(),
            name: "group_separator",
            aliases: vec!["group_sep", "gs"],
        },
        CharGroup {
            ch: '\x1e'.to_string(),
            name: "record_separator",
            aliases: vec!["record_sep", "rs"],
        },
        CharGroup {
            ch: '\x1f'.to_string(),
            name: "unit_separator",
            aliases: vec!["unit_sep", "us"],
        },
    ]
});

/// a lookup table from a name/alias to the character.
static CHAR_MAP: LazyLock<IndexMap<&'static str, String>> = LazyLock::new(|| {
    use std::iter::once;

    let mut map = IndexMap::new();

    for group in CHAR_GROUPS.iter() {
        for name in once(&group.name).chain(&group.aliases) {
            map.insert(*name, group.ch.to_owned());
        }
    }

    map
});

static CHAR_NAMES: LazyLock<Vec<&'static str>> =
    LazyLock::new(|| CHAR_MAP.keys().copied().collect());

impl Command for Char {
    fn name(&self) -> &str {
        "char"
    }

    fn signature(&self) -> Signature {
        Signature::build("char")
            .input_output_types(vec![(Type::Nothing, Type::Any)])
            .param(Parameter::Optional(
                PositionalArg::new("character", SyntaxShape::Any)
                    .desc("The name of the character to output.")
                    .completion(Completion::new_list(&CHAR_NAMES)),
            ))
            .rest("rest", SyntaxShape::Any, "Multiple Unicode bytes.")
            .switch("list", "List all supported character names.", Some('l'))
            .switch("unicode", "Unicode string i.e. 1f378.", Some('u'))
            .switch("integer", "Create a codepoint from an integer.", Some('i'))
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

    fn examples(&self) -> Vec<Example<'_>> {
        vec![
            Example {
                description: "Output newline",
                example: "char newline",
                result: Some(Value::test_string("\n")),
            },
            Example {
                description: "List available characters",
                example: "char --list",
                result: None,
            },
            Example {
                description: "Output prompt character, newline and a hamburger menu character",
                example: "(char prompt) + (char newline) + (char hamburger)",
                result: Some(Value::test_string("\u{25b6}\n\u{2261}")),
            },
            Example {
                description: "Output Unicode character",
                example: "char --unicode 1f378",
                result: Some(Value::test_string("\u{1f378}")),
            },
            Example {
                description: "Create Unicode from integer codepoint values",
                example: "char --integer (0x60 + 1) (0x60 + 2)",
                result: Some(Value::test_string("ab")),
            },
            Example {
                description: "Output multi-byte Unicode character",
                example: "char --unicode 1F468 200D 1F466 200D 1F466",
                result: Some(Value::test_string(
                    "\u{1F468}\u{200D}\u{1F466}\u{200D}\u{1F466}",
                )),
            },
        ]
    }

    fn run_const(
        &self,
        working_set: &StateWorkingSet,
        stack: &mut Stack,
        call: &Call,
        _input: PipelineData,
    ) -> Result<PipelineData, ShellError> {
        let call_span = call.head;
        let list = call.has_flag_const(working_set, stack, "list")?;
        let integer = call.has_flag_const(working_set, stack, "integer")?;
        let unicode = call.has_flag_const(working_set, stack, "unicode")?;

        // handle -l flag
        if list {
            return Ok(generate_character_list(
                working_set.permanent().signals().clone(),
                call.head,
            ));
        }

        // handle -i flag
        if integer {
            let int_args = call.rest_const(working_set, stack, 0)?;
            handle_integer_flag(int_args, call_span)
        }
        // handle -u flag
        else if unicode {
            let string_args = call.rest_const(working_set, stack, 0)?;
            handle_unicode_flag(string_args, call_span)
        }
        // handle the rest
        else {
            let string_args = call.rest_const(working_set, stack, 0)?;
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
    CHAR_GROUPS
        .iter()
        .map(move |group| {
            let name = Value::string(group.name, call_span);
            let aliases = group
                .aliases
                .iter()
                .map(|alias| Value::string(*alias, call_span))
                .collect();
            let aliases = Value::list(aliases, call_span);
            // control characters can make the list appear misaligned
            let character = if group.ch.chars().any(char::is_control) {
                Value::string("", call_span)
            } else {
                Value::string(&group.ch, call_span)
            };
            let unicode = Value::string(
                group
                    .ch
                    .chars()
                    .map(|c| format!("{:x}", c as u32))
                    .collect::<Vec<String>>()
                    .join(" "),
                call_span,
            );
            let record = record! {
                "name" => name,
                "character" => character,
                "unicode" => unicode,
                "aliases" => aliases,
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
    fn examples_work_as_expected() -> nu_test_support::Result {
        nu_test_support::test().examples(Char)
    }
}
