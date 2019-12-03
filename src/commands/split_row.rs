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
        args.process(registry, split_row)?.run()
    }
}

fn split_row(
    SplitRowArgs { separator }: SplitRowArgs,
    RunnableContext { input, name, .. }: RunnableContext,
) -> Result<OutputStream, ShellError> {
    let stream = input
        .values
        .map(move |v| {
            if let Ok(s) = v.as_string() {
                let splitter = separator.item.replace("\\n", "\n");
                trace!("splitting with {:?}", splitter);
                let split_result: Vec<_> = s.split(&splitter).filter(|s| s.trim() != "").collect();

                trace!("split result = {:?}", split_result);

                let mut result = VecDeque::new();
                for s in split_result {
                    result.push_back(ReturnSuccess::value(
                        UntaggedValue::Primitive(Primitive::String(s.into())).into_value(&v.tag),
                    ));
                }
                result
            } else {
                let mut result = VecDeque::new();
                result.push_back(Err(ShellError::labeled_error_with_secondary(
                    "Expected a string from pipeline",
                    "requires string input",
                    name.span,
                    "value originates from here",
                    v.tag.span,
                )));
                result
            }
        })
        .flatten();

    Ok(stream.to_output_stream())
}
