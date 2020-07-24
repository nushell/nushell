use crate::commands::WholeStreamCommand;
use crate::prelude::*;
use nu_errors::ShellError;
use nu_protocol::{ReturnSuccess, Signature, UntaggedValue};

pub struct Path;

#[async_trait]
impl WholeStreamCommand for Path {
    fn name(&self) -> &str {
        "path"
    }

    fn signature(&self) -> Signature {
        Signature::build("path")
    }

    fn usage(&self) -> &str {
        "Apply path function"
    }

    async fn run(
        &self,
        _args: CommandArgs,
        registry: &CommandRegistry,
    ) -> Result<OutputStream, ShellError> {
        let registry = registry.clone();

        Ok(OutputStream::one(ReturnSuccess::value(
            UntaggedValue::string(crate::commands::help::get_help(&Path, &registry))
                .into_value(Tag::unknown()),
        )))
    }
}

#[cfg(test)]
mod tests {
    use super::Path;

    #[test]
    fn examples_work_as_expected() {
        use crate::examples::test as test_examples;

        test_examples(Path {})
    }
}
