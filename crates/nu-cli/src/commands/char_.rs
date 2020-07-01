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
        vec![Example {
            description: "Output newline",
            example: r#"char newline"#,
            result: Some(vec![Value::from("\n")]),
        }]
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
        "branch" => Some('\u{e0a0}'.to_string()), // 
        "branch_identical" => Some('\u{2263}'.to_string()), // ≣
        "branch_untracked" => Some('\u{2262}'.to_string()), // ≢
        "branch_ahead" => Some('\u{2191}'.to_string()), // ↑
        "branch_behind" => Some('\u{2193}'.to_string()), // ↓
        "branch_ahead_behind" => Some('\u{2195}'.to_string()), // ↕
        "prompt" => Some('\u{25b6}'.to_string()), // ▶
        "failed" => Some('\u{2a2f}'.to_string()), // ⨯
        "elevated" => Some('\u{26a1}'.to_string()), // ⚡
        "segment" => Some('\u{e0b0}'.to_string()), // 
        "home" => Some("~".into()),               // ~
        "root" => Some("#".into()),               // #
        "hamburger" => Some('\u{2261}'.to_string()), // ≡
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::Char;

    #[test]
    fn examples_work_as_expected() {
        use crate::examples::test as test_examples;

        test_examples(Char {})
    }
}
