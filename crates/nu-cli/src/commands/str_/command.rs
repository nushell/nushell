use crate::commands::WholeStreamCommand;
use crate::prelude::*;
use nu_errors::ShellError;
use nu_protocol::{ReturnSuccess, Signature, SyntaxShape, UntaggedValue};

pub struct Command;

impl WholeStreamCommand for Command {
    fn name(&self) -> &str {
        "str"
    }

    fn signature(&self) -> Signature {
        Signature::build("str").rest(
            SyntaxShape::ColumnPath,
            "optionally convert by column paths",
        )
    }

    fn usage(&self) -> &str {
        "Apply string function."
    }

    fn run(
        &self,
        _args: CommandArgs,
        registry: &CommandRegistry,
    ) -> Result<OutputStream, ShellError> {
        let registry = registry.clone();
        let stream = async_stream! {
            yield Ok(ReturnSuccess::Value(
                UntaggedValue::string(crate::commands::help::get_help(&Command, &registry))
                    .into_value(Tag::unknown()),
            ));
        };

        Ok(stream.to_output_stream())
    }
}

#[cfg(test)]
mod tests {
    use super::Command;

    #[test]
    fn examples_work_as_expected() {
        use crate::examples::test as test_examples;

        test_examples(Command {})
    }
}
