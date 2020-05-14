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

pub struct SplitRow;

impl WholeStreamCommand for SplitRow {
    fn name(&self) -> &str {
        "split-row"
    }

    fn signature(&self) -> Signature {
        Signature::build("split-row").required(
            "separator",
            SyntaxShape::Any,
            "the character that denotes what separates rows",
        )
    }

    fn usage(&self) -> &str {
        "Split row contents over multiple rows via the separator."
    }

    fn run(
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
        let input = args.input;
        let name = args.call_info.name_tag.clone();
        let SplitRowArgs { separator } = args.process_raw(&registry).await?;
        for v in input.next().await {
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
