use crate::prelude::*;
use nu_engine::WholeStreamCommand;
use nu_errors::ShellError;
use nu_protocol::{Signature, UntaggedValue};

pub struct Path;

impl WholeStreamCommand for Path {
    fn name(&self) -> &str {
        "path"
    }

    fn signature(&self) -> Signature {
        Signature::build("path")
    }

    fn usage(&self) -> &str {
        "Explore and manipulate paths."
    }

    fn extra_usage(&self) -> &str {
        r#"There are three ways to represent a path:

* As a path literal, e.g., '/home/viking/spam.txt'
* As a structured path: a table with 'parent', 'stem', and 'extension' (and
* 'prefix' on Windows) columns. This format is produced by the 'path parse'
  subcommand.
* As an inner list of path parts, e.g., '[[ / home viking spam.txt ]]'.
  Splitting into parts is done by the `path split` command.

All subcommands accept all three variants as an input. Furthermore, the 'path
join' subcommand can be used to join the structured path or path parts back into
the path literal."#
    }

    fn run(&self, args: CommandArgs) -> Result<OutputStream, ShellError> {
        Ok(OutputStream::one(
            UntaggedValue::string(get_full_help(&Path, args.scope())).into_value(Tag::unknown()),
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::Path;
    use super::ShellError;

    #[test]
    fn examples_work_as_expected() -> Result<(), ShellError> {
        use crate::examples::test as test_examples;

        test_examples(Path {})
    }
}
