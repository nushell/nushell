use crate::prelude::*;
use nu_engine::WholeStreamCommand;
use nu_errors::ShellError;
use nu_protocol::{ReturnSuccess, Signature, UntaggedValue};

#[derive(Clone)]
pub struct Chart;

#[async_trait]
impl WholeStreamCommand for Chart {
    fn name(&self) -> &str {
        "chart"
    }

    fn signature(&self) -> Signature {
        Signature::build("chart")
    }

    fn usage(&self) -> &str {
        "Displays charts."
    }

    async fn run(&self, args: CommandArgs) -> Result<OutputStream, ShellError> {
        if args.scope.get_command("chart bar").is_none() {
            return Err(ShellError::untagged_runtime_error(
                "nu_plugin_chart not installed.",
            ));
        }

        Ok(OutputStream::one(Ok(ReturnSuccess::Value(
            UntaggedValue::string(get_help(&Chart, &args.scope)).into_value(Tag::unknown()),
        ))))
    }
}

#[cfg(test)]
mod tests {
    use super::Chart;
    use super::ShellError;

    #[test]
    fn examples_work_as_expected() -> Result<(), ShellError> {
        use crate::examples::test as test_examples;

        test_examples(Chart {})
    }
}
