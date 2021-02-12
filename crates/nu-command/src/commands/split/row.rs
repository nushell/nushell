use crate::prelude::*;
use log::trace;
use nu_engine::WholeStreamCommand;
use nu_errors::ShellError;
use nu_protocol::{Primitive, ReturnSuccess, Signature, SyntaxShape, UntaggedValue};
use nu_source::Tagged;

#[derive(Deserialize)]
struct SplitRowArgs {
    separator: Tagged<String>,
}

pub struct SubCommand;

#[async_trait]
impl WholeStreamCommand for SubCommand {
    fn name(&self) -> &str {
        "split row"
    }

    fn signature(&self) -> Signature {
        Signature::build("split row").required(
            "separator",
            SyntaxShape::Any,
            "the character that denotes what separates rows",
        )
    }

    fn usage(&self) -> &str {
        "splits contents over multiple rows via the separator."
    }

    async fn run(&self, args: CommandArgs) -> Result<OutputStream, ShellError> {
        split_row(args).await
    }
}

async fn split_row(args: CommandArgs) -> Result<OutputStream, ShellError> {
    let name = args.call_info.name_tag.clone();
    let (SplitRowArgs { separator }, input) = args.process().await?;
    Ok(input
        .flat_map(move |v| {
            if let Ok(s) = v.as_string() {
                let splitter = separator.item.replace("\\n", "\n");
                trace!("splitting with {:?}", splitter);
                let split_result: Vec<String> = s
                    .split(&splitter)
                    .filter_map(|s| {
                        if s.trim() != "" {
                            Some(s.to_string())
                        } else {
                            None
                        }
                    })
                    .collect();

                trace!("split result = {:?}", split_result);

                futures::stream::iter(split_result.into_iter().map(move |s| {
                    ReturnSuccess::value(
                        UntaggedValue::Primitive(Primitive::String(s)).into_value(&v.tag),
                    )
                }))
                .to_output_stream()
            } else {
                OutputStream::one(Err(ShellError::labeled_error_with_secondary(
                    "Expected a string from pipeline",
                    "requires string input",
                    name.span,
                    "value originates from here",
                    v.tag.span,
                )))
            }
        })
        .to_output_stream())
}

#[cfg(test)]
mod tests {
    use super::ShellError;
    use super::SubCommand;

    #[test]
    fn examples_work_as_expected() -> Result<(), ShellError> {
        use crate::examples::test as test_examples;

        test_examples(SubCommand {})
    }
}
