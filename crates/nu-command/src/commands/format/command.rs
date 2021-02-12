use crate::prelude::*;
use nu_engine::evaluate_baseline_expr;
use nu_engine::WholeStreamCommand;
use nu_errors::ShellError;
use nu_protocol::{ReturnSuccess, Signature, SyntaxShape, UntaggedValue};
use nu_source::Tagged;
use std::borrow::Borrow;

pub struct Format;

#[derive(Deserialize)]
pub struct FormatArgs {
    pattern: Tagged<String>,
}

#[async_trait]
impl WholeStreamCommand for Format {
    fn name(&self) -> &str {
        "format"
    }

    fn signature(&self) -> Signature {
        Signature::build("format").required(
            "pattern",
            SyntaxShape::String,
            "the pattern to output. Eg) \"{foo}: {bar}\"",
        )
    }

    fn usage(&self) -> &str {
        "Format columns into a string using a simple pattern."
    }

    async fn run(&self, args: CommandArgs) -> Result<OutputStream, ShellError> {
        format_command(args).await
    }

    fn examples(&self) -> Vec<Example> {
        vec![Example {
            description: "Print filenames with their sizes",
            example: "ls | format '{name}: {size}'",
            result: None,
        }]
    }
}

async fn format_command(args: CommandArgs) -> Result<OutputStream, ShellError> {
    let ctx = Arc::new(EvaluationContext::from_args(&args));
    let (FormatArgs { pattern }, input) = args.process().await?;

    let format_pattern = format(&pattern);
    let commands = Arc::new(format_pattern);

    Ok(input
        .then(move |value| {
            let mut output = String::new();
            let commands = commands.clone();
            let ctx = ctx.clone();

            async move {
                for command in &*commands {
                    match command {
                        FormatCommand::Text(s) => {
                            output.push_str(&s);
                        }
                        FormatCommand::Column(c) => {
                            // FIXME: use the correct spans
                            let full_column_path = nu_parser::parse_full_column_path(
                                &(c.to_string()).spanned(Span::unknown()),
                                &ctx.scope,
                            );

                            ctx.scope.enter_scope();
                            ctx.scope.add_var("$it", value.clone());
                            let result = evaluate_baseline_expr(&full_column_path.0, &*ctx).await;
                            ctx.scope.exit_scope();

                            if let Ok(c) = result {
                                output
                                    .push_str(&value::format_leaf(c.borrow()).plain_string(100_000))
                            } else {
                                // That column doesn't match, so don't emit anything
                            }
                        }
                    }
                }

                ReturnSuccess::value(UntaggedValue::string(output).into_untagged_value())
            }
        })
        .to_output_stream())
}

#[derive(Debug)]
enum FormatCommand {
    Text(String),
    Column(String),
}

fn format(input: &str) -> Vec<FormatCommand> {
    let mut output = vec![];

    let mut loop_input = input.chars();
    loop {
        let mut before = String::new();

        while let Some(c) = loop_input.next() {
            if c == '{' {
                break;
            }
            before.push(c);
        }

        if !before.is_empty() {
            output.push(FormatCommand::Text(before.to_string()));
        }
        // Look for column as we're now at one
        let mut column = String::new();

        while let Some(c) = loop_input.next() {
            if c == '}' {
                break;
            }
            column.push(c);
        }

        if !column.is_empty() {
            output.push(FormatCommand::Column(column.to_string()));
        }

        if before.is_empty() && column.is_empty() {
            break;
        }
    }

    output
}

#[cfg(test)]
mod tests {
    use super::Format;
    use super::ShellError;

    #[test]
    fn examples_work_as_expected() -> Result<(), ShellError> {
        use crate::examples::test as test_examples;

        test_examples(Format {})
    }
}
