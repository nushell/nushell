use crate::commands::WholeStreamCommand;
use crate::prelude::*;
use nu_errors::ShellError;
use nu_protocol::{ReturnSuccess, Signature, UntaggedValue};

#[derive(Clone)]
pub struct To;

impl WholeStreamCommand for To {
    fn name(&self) -> &str {
        "to"
    }

    fn signature(&self) -> Signature {
        Signature::build("to")
    }

    fn usage(&self) -> &str {
        "Convert table into an output format (based on subcommand, like csv, html, json, yaml)."
    }

    fn run(
        &self,
        _args: CommandArgs,
        registry: &CommandRegistry,
    ) -> Result<OutputStream, ShellError> {
        let registry = registry.clone();
        let stream = async_stream! {
            yield Ok(ReturnSuccess::Value(
                UntaggedValue::string(crate::commands::help::get_help(&To, &registry))
                    .into_value(Tag::unknown()),
            ));
        };

        Ok(stream.to_output_stream())
    }
}

#[cfg(test)]
mod tests {
    use super::To;

    #[test]
    fn examples_work_as_expected() {
        use crate::examples::test as test_examples;

        test_examples(To {})
    }
}
