use crate::formats::is_dotenv_key;
use nu_engine::command_prelude::*;
use nu_engine::env::env_to_string;
use nu_protocol::shell_error::generic::GenericError;

#[derive(Clone)]
pub struct ToDotenv;

impl Command for ToDotenv {
    fn name(&self) -> &str {
        "to dotenv"
    }

    fn signature(&self) -> Signature {
        Signature::build("to dotenv")
            .input_output_types(vec![(Type::record(), Type::String)])
            .category(Category::Formats)
    }

    fn description(&self) -> &str {
        "Convert a record into .env text."
    }

    fn extra_description(&self) -> &str {
        "Each value is written as an external command would see it, so a `PATH` list is joined \
with the path separator and the `to_string` conversions in `$env.ENV_CONVERSIONS` apply, and \
`null` is written as an empty value. A value of plain characters is written bare; any other is \
single-quoted, or double-quoted with escapes when it holds a `'` or a line break, so \
`from dotenv` reads back the same text. `save` writes a record to a file named like `.env`, \
`.env.local` or `prod.env` with this command."
    }

    fn search_terms(&self) -> Vec<&str> {
        vec!["environment", ".env"]
    }

    fn run(
        &self,
        engine_state: &EngineState,
        stack: &mut Stack,
        call: &Call,
        mut input: PipelineData,
    ) -> Result<PipelineData, ShellError> {
        let head = call.head;
        let metadata = input.take_metadata();
        let value = input.into_value(head)?;
        // `$env`'s values carry no place in the source, so their errors point at the command.
        let known = |span: Span| if span == Span::unknown() { head } else { span };
        let span = known(value.span());
        let record = value.into_record()?;

        let mut text = String::new();
        for (key, value) in &record {
            if !is_dotenv_key(key) {
                return Err(ShellError::Generic(
                    GenericError::new(
                        format!("Can't write `{key}` as .env text"),
                        "not a .env variable name",
                        span,
                    )
                    .with_help(
                        "a name holds letters, digits, `_`, `.` and `-`, and starts with a \
                         letter or `_`",
                    ),
                ));
            }
            // A missing value, as `select -o` gives, is written empty.
            let value = if let Value::Nothing { .. } = value {
                String::new()
            } else {
                env_to_string(key, value, engine_state, stack).map_err(|err| match err {
                    ShellError::EnvVarNotAString { .. } => ShellError::Generic(
                        GenericError::new(
                            format!("Can't write `{key}` as .env text"),
                            format!(
                                "{} has no text form",
                                value.get_type().get_non_specified_string()
                            ),
                            known(value.span()),
                        )
                        .with_help(
                            "select the variables to write, or convert this one to a string",
                        ),
                    ),
                    err => err,
                })?
            };
            text.push_str(key);
            text.push('=');
            push_value(&mut text, &value);
            text.push('\n');
        }

        Ok(Value::string(text, head)
            .into_pipeline_data_with_metadata(metadata.map(|md| md.with_content_type(None))))
    }

    fn examples(&self) -> Vec<Example<'_>> {
        vec![
            Example {
                description: "Write a record as .env text, quoting values that need it.",
                example: "{PORT: 8080, NAME: 'my app', GREETING: \"it's\"} | to dotenv",
                result: Some(Value::test_string(
                    "PORT=8080\nNAME='my app'\nGREETING=\"it's\"\n",
                )),
            },
            Example {
                description: "Round-trip a record through .env text.",
                example: r#"{URL: 'http://${HOST}/', NOTE: "two\nlines"} | to dotenv | from dotenv"#,
                result: Some(Value::test_record(record! {
                    "URL" => Value::test_string("http://${HOST}/"),
                    "NOTE" => Value::test_string("two\nlines"),
                })),
            },
            Example {
                description: "Save a record to a .env file.",
                example: "{PORT: 8080, NAME: 'my app'} | save .env",
                result: None,
            },
        ]
    }
}

/// Appends `value` as .env text that `from dotenv` reads back as `value`: bare when every
/// character is plain, single-quoted when it has no `'` or line break, and otherwise
/// double-quoted with `\` escapes.
fn push_value(text: &mut String, value: &str) {
    let plain = |c: char| c.is_alphanumeric() || "_-./:@%+,=".contains(c);
    if value.chars().all(plain) {
        text.push_str(value);
    } else if !value.contains(['\'', '\n', '\r']) {
        text.push('\'');
        text.push_str(value);
        text.push('\'');
    } else {
        text.push('"');
        let mut chars = value.chars().peekable();
        while let Some(c) = chars.next() {
            match c {
                '"' | '\\' => {
                    text.push('\\');
                    text.push(c);
                }
                // `${NAME}` and `$NAME` expand, so a `$` before `{` or a name is escaped, and
                // any other stays as it is.
                '$' if chars.peek().is_some_and(|&next| {
                    next == '{' || next == '_' || next.is_ascii_alphabetic()
                }) =>
                {
                    text.push_str("\\$")
                }
                '\n' => text.push_str("\\n"),
                '\r' => text.push_str("\\r"),
                c => text.push(c),
            }
        }
        text.push('"');
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn examples_work_as_expected() -> nu_test_support::Result {
        nu_test_support::test().examples(ToDotenv)
    }

    fn written(value: &str) -> String {
        let mut text = String::new();
        push_value(&mut text, value);
        text
    }

    #[test]
    fn values_are_quoted_only_when_needed() {
        assert_eq!(written("/usr/bin:/bin"), "/usr/bin:/bin");
        assert_eq!(written(""), "");
        assert_eq!(written("a b"), "'a b'");
        assert_eq!(written("x #y"), "'x #y'");
        assert_eq!(written("${HOME}"), "'${HOME}'");
        assert_eq!(written(r"C:\bin;C:\tools"), r"'C:\bin;C:\tools'");
        assert_eq!(written("it's ${A} \\ \"q\""), r#""it's \${A} \\ \"q\"""#);
        assert_eq!(written("cost $5\r\n"), r#""cost $5\r\n""#);
        assert_eq!(written("it's $HOME $_x $5 $"), r#""it's \$HOME \$_x $5 $""#);
    }
}
