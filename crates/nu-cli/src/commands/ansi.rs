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
            Example {
                description:
                    "Use ansi to color text (rb = red bold, gb = green bold, pb = purple bold)",
                example: r#"echo [$(ansi rb) Hello " " $(ansi gb) Nu " " $(ansi pb) World] | str collect"#,
                result: Some(vec![Value::from(
                    "\u{1b}[1;31mHello \u{1b}[1;32mNu \u{1b}[1;35mWorld",
                )]),
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
        "gb" | "green_bold" => Some(ansi_term::Color::Green.bold().prefix().to_string()),
        "gu" | "green_underline" => Some(ansi_term::Color::Green.underline().prefix().to_string()),
        "gi" | "green_italic" => Some(ansi_term::Color::Green.italic().prefix().to_string()),
        "gd" | "green_dimmed" => Some(ansi_term::Color::Green.dimmed().prefix().to_string()),
        "gr" | "green_reverse" => Some(ansi_term::Color::Green.reverse().prefix().to_string()),
        "r" | "red" => Some(ansi_term::Color::Red.prefix().to_string()),
        "rb" | "red_bold" => Some(ansi_term::Color::Red.bold().prefix().to_string()),
        "ru" | "red_underline" => Some(ansi_term::Color::Red.underline().prefix().to_string()),
        "ri" | "red_italic" => Some(ansi_term::Color::Red.italic().prefix().to_string()),
        "rd" | "red_dimmed" => Some(ansi_term::Color::Red.dimmed().prefix().to_string()),
        "rr" | "red_reverse" => Some(ansi_term::Color::Red.reverse().prefix().to_string()),
        "u" | "blue" => Some(ansi_term::Color::Blue.prefix().to_string()),
        "ub" | "blue_bold" => Some(ansi_term::Color::Blue.bold().prefix().to_string()),
        "uu" | "blue_underline" => Some(ansi_term::Color::Blue.underline().prefix().to_string()),
        "ui" | "blue_italic" => Some(ansi_term::Color::Blue.italic().prefix().to_string()),
        "ud" | "blue_dimmed" => Some(ansi_term::Color::Blue.dimmed().prefix().to_string()),
        "ur" | "blue_reverse" => Some(ansi_term::Color::Blue.reverse().prefix().to_string()),
        "b" | "black" => Some(ansi_term::Color::Black.prefix().to_string()),
        "bb" | "black_bold" => Some(ansi_term::Color::Black.bold().prefix().to_string()),
        "bu" | "black_underline" => Some(ansi_term::Color::Black.underline().prefix().to_string()),
        "bi" | "black_italic" => Some(ansi_term::Color::Black.italic().prefix().to_string()),
        "bd" | "black_dimmed" => Some(ansi_term::Color::Black.dimmed().prefix().to_string()),
        "br" | "black_reverse" => Some(ansi_term::Color::Black.reverse().prefix().to_string()),
        "y" | "yellow" => Some(ansi_term::Color::Yellow.prefix().to_string()),
        "yb" | "yellow_bold" => Some(ansi_term::Color::Yellow.bold().prefix().to_string()),
        "yu" | "yellow_underline" => {
            Some(ansi_term::Color::Yellow.underline().prefix().to_string())
        }
        "yi" | "yellow_italic" => Some(ansi_term::Color::Yellow.italic().prefix().to_string()),
        "yd" | "yellow_dimmed" => Some(ansi_term::Color::Yellow.dimmed().prefix().to_string()),
        "yr" | "yellow_reverse" => Some(ansi_term::Color::Yellow.reverse().prefix().to_string()),
        "p" | "purple" => Some(ansi_term::Color::Purple.prefix().to_string()),
        "pb" | "purple_bold" => Some(ansi_term::Color::Purple.bold().prefix().to_string()),
        "pu" | "purple_underline" => {
            Some(ansi_term::Color::Purple.underline().prefix().to_string())
        }
        "pi" | "purple_italic" => Some(ansi_term::Color::Purple.italic().prefix().to_string()),
        "pd" | "purple_dimmed" => Some(ansi_term::Color::Purple.dimmed().prefix().to_string()),
        "pr" | "purple_reverse" => Some(ansi_term::Color::Purple.reverse().prefix().to_string()),
        "c" | "cyan" => Some(ansi_term::Color::Cyan.prefix().to_string()),
        "cb" | "cyan_bold" => Some(ansi_term::Color::Cyan.bold().prefix().to_string()),
        "cu" | "cyan_underline" => Some(ansi_term::Color::Cyan.underline().prefix().to_string()),
        "ci" | "cyan_italic" => Some(ansi_term::Color::Cyan.italic().prefix().to_string()),
        "cd" | "cyan_dimmed" => Some(ansi_term::Color::Cyan.dimmed().prefix().to_string()),
        "cr" | "cyan_reverse" => Some(ansi_term::Color::Cyan.reverse().prefix().to_string()),
        "w" | "white" => Some(ansi_term::Color::White.prefix().to_string()),
        "wb" | "white_bold" => Some(ansi_term::Color::White.bold().prefix().to_string()),
        "wu" | "white_underline" => Some(ansi_term::Color::White.underline().prefix().to_string()),
        "wi" | "white_italic" => Some(ansi_term::Color::White.italic().prefix().to_string()),
        "wd" | "white_dimmed" => Some(ansi_term::Color::White.dimmed().prefix().to_string()),
        "wr" | "white_reverse" => Some(ansi_term::Color::White.reverse().prefix().to_string()),
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
