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
        "Explore and manipulate paths"
    }

    async fn run(&self, args: CommandArgs) -> Result<OutputStream, ShellError> {
        Ok(OutputStream::one(ReturnSuccess::value(
            UntaggedValue::string(crate::commands::help::get_help(&Path, &args.scope))
                .into_value(Tag::unknown()),
        )))
    }
}

#[cfg(test)]
mod tests {
    use super::Path;
    use super::ShellError;

    #[test]
    fn examples_work_as_expected() -> Result<(), ShellError> {
        use crate::examples::test as test_examples;

        Ok(test_examples(Path {})?)
    }
}
