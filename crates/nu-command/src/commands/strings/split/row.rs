use crate::prelude::*;
use log::trace;
use nu_engine::WholeStreamCommand;
use nu_errors::ShellError;
use nu_protocol::{Primitive, ReturnSuccess, Signature, SyntaxShape, UntaggedValue};
use nu_source::Tagged;

pub struct SubCommand;

impl WholeStreamCommand for SubCommand {
    fn name(&self) -> &str {
        "split row"
    }

    fn signature(&self) -> Signature {
        Signature::build("split row").required(
            "separator",
            SyntaxShape::String,
            "the character that denotes what separates rows",
        )
    }

    fn usage(&self) -> &str {
        "splits contents over multiple rows via the separator."
    }

    fn run_with_actions(&self, args: CommandArgs) -> Result<ActionStream, ShellError> {
        split_row(args)
    }
}

fn split_row(args: CommandArgs) -> Result<ActionStream, ShellError> {
    let name = args.call_info.name_tag.clone();

    let separator: Tagged<String> = args.req(0)?;
    let input = args.input;

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

                (split_result.into_iter().map(move |s| {
                    ReturnSuccess::value(
                        UntaggedValue::Primitive(Primitive::String(s)).into_value(&v.tag),
                    )
                }))
                .into_action_stream()
            } else {
                ActionStream::one(Err(ShellError::labeled_error_with_secondary(
                    "Expected a string from pipeline",
                    "requires string input",
                    name.span,
                    "value originates from here",
                    v.tag.span,
                )))
            }
        })
        .into_action_stream())
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
