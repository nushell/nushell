use crate::prelude::*;
use nu_engine::WholeStreamCommand;
use nu_errors::ShellError;
use nu_protocol::{Signature, SyntaxShape, UntaggedValue};
use nu_source::Tagged;
use rand::distributions::Alphanumeric;
use rand::prelude::{thread_rng, Rng};

pub struct SubCommand;

pub struct CharsArgs {
    length: Option<Tagged<u32>>,
}

const DEFAULT_CHARS_LENGTH: u32 = 25;

impl WholeStreamCommand for SubCommand {
    fn name(&self) -> &str {
        "random chars"
    }

    fn signature(&self) -> Signature {
        Signature::build("random chars").named(
            "length",
            SyntaxShape::Int,
            "Number of chars",
            Some('l'),
        )
    }

    fn usage(&self) -> &str {
        "Generate random chars"
    }

    fn run(&self, args: CommandArgs) -> Result<OutputStream, ShellError> {
        chars(args)
    }

    fn examples(&self) -> Vec<Example> {
        vec![
            Example {
                description: "Generate random chars",
                example: "random chars",
                result: None,
            },
            Example {
                description: "Generate random chars with specified length",
                example: "random chars -l 20",
                result: None,
            },
        ]
    }
}

pub fn chars(args: CommandArgs) -> Result<OutputStream, ShellError> {
    let cmd_args = CharsArgs {
        length: args.get_flag("length")?,
    };

    let chars_length = cmd_args.length.map_or(DEFAULT_CHARS_LENGTH, |l| l.item);

    let random_string: String = thread_rng()
        .sample_iter(&Alphanumeric)
        .take(chars_length as usize)
        .collect();

    Ok(OutputStream::one(UntaggedValue::string(random_string)))
}

#[cfg(test)]
mod tests {
    use super::ShellError;
    use super::SubCommand;

    #[test]
    fn examples_work_as_expected() -> Result<(), ShellError> {
        use crate::examples::test as test_examples;

        test_examples(SubCommand {})
    }
}
