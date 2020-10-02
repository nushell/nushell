use crate::commands::WholeStreamCommand;
use crate::prelude::*;
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

    async fn run(
        &self,
        _args: CommandArgs,
        registry: &CommandRegistry,
    ) -> Result<OutputStream, ShellError> {
        if registry.get_command("chart bar").is_none() {
            return Err(ShellError::untagged_runtime_error(
                "nu_plugin_chart not installed.",
            ));
        }

        let registry = registry.clone();
        Ok(OutputStream::one(Ok(ReturnSuccess::Value(
            UntaggedValue::string(crate::commands::help::get_help(&Chart, &registry))
                .into_value(Tag::unknown()),
        ))))
    }
}

#[cfg(test)]
mod tests {
    use super::Chart;

    #[test]
    fn examples_work_as_expected() {
        use crate::examples::test as test_examples;

        test_examples(Chart {})
    }
}
