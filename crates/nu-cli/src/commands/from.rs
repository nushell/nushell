use crate::commands::WholeStreamCommand;
use crate::prelude::*;
use nu_errors::ShellError;
use nu_protocol::{ReturnSuccess, Signature, UntaggedValue};

pub struct From;

impl WholeStreamCommand for From {
    fn name(&self) -> &str {
        "from"
    }

    fn signature(&self) -> Signature {
        Signature::build("from")
    }

    fn usage(&self) -> &str {
        "Parse content (string or binary) as a table (input format based on subcommand, like csv, ini, json, toml)"
    }

    fn run(
        &self,
        _args: CommandArgs,
        registry: &CommandRegistry,
    ) -> Result<OutputStream, ShellError> {
        let registry = registry.clone();
        let stream = async_stream! {
            yield Ok(ReturnSuccess::Value(
                UntaggedValue::string(crate::commands::help::get_help(&From, &registry))
                    .into_value(Tag::unknown()),
            ));
        };

        Ok(stream.to_output_stream())
    }
}

#[cfg(test)]
mod tests {
    use super::From;

    #[test]
    fn examples_work_as_expected() {
        use crate::examples::test as test_examples;

        test_examples(From {})
    }
}
