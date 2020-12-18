use crate::commands::WholeStreamCommand;
use crate::prelude::*;
use nu_errors::ShellError;
use nu_protocol::{ReturnSuccess, Signature, UntaggedValue};

#[derive(Clone)]
pub struct To;

#[async_trait]
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

    async fn run(&self, args: CommandArgs) -> Result<OutputStream, ShellError> {
        Ok(OutputStream::one(ReturnSuccess::value(
            UntaggedValue::string(crate::commands::help::get_help(&To, &args.scope))
                .into_value(Tag::unknown()),
        )))
    }
}

#[cfg(test)]
mod tests {
    use super::ShellError;
    use super::To;

    #[test]
    fn examples_work_as_expected() -> Result<(), ShellError> {
        use crate::examples::test as test_examples;

        Ok(test_examples(To {})?)
    }
}
