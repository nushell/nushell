use crate::commands::WholeStreamCommand;
use crate::prelude::*;
use log::trace;
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

    async fn run(
        &self,
        args: CommandArgs,
        registry: &CommandRegistry,
    ) -> Result<OutputStream, ShellError> {
        split_row(args, registry)
    }
}

fn split_row(args: CommandArgs, registry: &CommandRegistry) -> Result<OutputStream, ShellError> {
    let registry = registry.clone();
    let stream = async_stream! {
        let name = args.call_info.name_tag.clone();
        let (SplitRowArgs { separator }, mut input) = args.process(&registry).await?;
        while let Some(v) = input.next().await {
            if let Ok(s) = v.as_string() {
                let splitter = separator.item.replace("\\n", "\n");
                trace!("splitting with {:?}", splitter);
                let split_result: Vec<_> = s.split(&splitter).filter(|s| s.trim() != "").collect();

                trace!("split result = {:?}", split_result);

                for s in split_result {
                    yield ReturnSuccess::value(
                        UntaggedValue::Primitive(Primitive::String(s.into())).into_value(&v.tag),
                    );
                }
            } else {
                yield Err(ShellError::labeled_error_with_secondary(
                    "Expected a string from pipeline",
                    "requires string input",
                    name.span,
                    "value originates from here",
                    v.tag.span,
                ));
            }
        }
    };

    Ok(stream.to_output_stream())
}

#[cfg(test)]
mod tests {
    use super::SubCommand;

    #[test]
    fn examples_work_as_expected() {
        use crate::examples::test as test_examples;

        test_examples(SubCommand {})
    }
}
