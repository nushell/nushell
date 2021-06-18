extern crate unicode_segmentation;

use crate::prelude::*;
use nu_engine::WholeStreamCommand;
use nu_errors::ShellError;
use nu_protocol::Signature;

pub struct Ignore;

impl WholeStreamCommand for Ignore {
    fn name(&self) -> &str {
        "ignore"
    }

    fn signature(&self) -> Signature {
        Signature::build("ignore")
    }

    fn usage(&self) -> &str {
        "Ignore the output of the previous command in the pipeline"
    }

    fn run(&self, args: CommandArgs) -> Result<OutputStream, ShellError> {
        let _: Vec<_> = args.input.collect();

        Ok(OutputStream::empty())
    }

    fn examples(&self) -> Vec<Example> {
        vec![Example {
            description: "echo done | ignore",
            example: r#"echo "There are seven words in this sentence" | size"#,
            result: None,
        }]
    }
}

#[cfg(test)]
mod tests {
    use super::Ignore;
    use super::ShellError;

    #[test]
    fn examples_work_as_expected() -> Result<(), ShellError> {
        use crate::examples::test as test_examples;

        test_examples(Ignore {})
    }
}
