use crate::commands::WholeStreamCommand;
use crate::prelude::*;
use nu_errors::ShellError;
use nu_protocol::{ReturnSuccess, Signature, SyntaxShape, UntaggedValue, Value};

pub struct Ansi;

#[derive(Deserialize)]
struct AnsiArgs {
    color: Value,
}

#[async_trait]
impl WholeStreamCommand for Ansi {
    fn name(&self) -> &str {
        "ansi"
    }

    fn signature(&self) -> Signature {
        Signature::build("ansi").required(
            "color",
            SyntaxShape::Any,
            "the name of the color to use or 'reset' to reset the color",
        )
    }

    fn usage(&self) -> &str {
        "Output ANSI codes to change color"
    }

    fn examples(&self) -> Vec<Example> {
        vec![
            Example {
                description: "Change color to green",
                example: r#"ansi green"#,
                result: Some(vec![Value::from("\u{1b}[32m")]),
            },
            Example {
                description: "Reset the color",
                example: r#"ansi reset"#,
                result: Some(vec![Value::from("\u{1b}[0m")]),
            },
        ]
    }

    async fn run(
        &self,
        args: CommandArgs,
        registry: &CommandRegistry,
    ) -> Result<OutputStream, ShellError> {
        let (AnsiArgs { color }, _) = args.process(&registry).await?;

        let color_string = color.as_string()?;

        let ansi_code = str_to_ansi_color(color_string);

        if let Some(output) = ansi_code {
            Ok(OutputStream::one(ReturnSuccess::value(
                UntaggedValue::string(output).into_value(color.tag()),
            )))
        } else {
            Err(ShellError::labeled_error(
                "Unknown color",
                "unknown color",
                color.tag(),
            ))
        }
    }
}

fn str_to_ansi_color(s: String) -> Option<String> {
    match s.as_str() {
        "g" | "green" => Some(ansi_term::Color::Green.prefix().to_string()),
        "r" | "red" => Some(ansi_term::Color::Red.prefix().to_string()),
        "u" | "blue" => Some(ansi_term::Color::Blue.prefix().to_string()),
        "b" | "black" => Some(ansi_term::Color::Black.prefix().to_string()),
        "y" | "yellow" => Some(ansi_term::Color::Yellow.prefix().to_string()),
        "p" | "purple" => Some(ansi_term::Color::Purple.prefix().to_string()),
        "c" | "cyan" => Some(ansi_term::Color::Cyan.prefix().to_string()),
        "w" | "white" => Some(ansi_term::Color::White.prefix().to_string()),
        "reset" => Some("\x1b[0m".to_owned()),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::Ansi;

    #[test]
    fn examples_work_as_expected() {
        use crate::examples::test as test_examples;

        test_examples(Ansi {})
    }
}
