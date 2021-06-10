use crate::prelude::*;
use nu_engine::WholeStreamCommand;
use nu_errors::ShellError;
use nu_protocol::{Signature, SyntaxShape, UntaggedValue};
use nu_source::Tagged;
use rand::prelude::{thread_rng, Rng};

pub struct SubCommand;

pub struct BoolArgs {
    bias: Option<Tagged<f64>>,
}

impl WholeStreamCommand for SubCommand {
    fn name(&self) -> &str {
        "random bool"
    }

    fn signature(&self) -> Signature {
        Signature::build("random bool").named(
            "bias",
            SyntaxShape::Number,
            "Adjusts the probability of a \"true\" outcome",
            Some('b'),
        )
    }

    fn usage(&self) -> &str {
        "Generate a random boolean value"
    }

    fn run(&self, args: CommandArgs) -> Result<OutputStream, ShellError> {
        bool_command(args)
    }

    fn examples(&self) -> Vec<Example> {
        vec![
            Example {
                description: "Generate a random boolean value",
                example: "random bool",
                result: None,
            },
            Example {
                description: "Generate a random boolean value with a 75% chance of \"true\"",
                example: "random bool --bias 0.75",
                result: None,
            },
        ]
    }
}

pub fn bool_command(args: CommandArgs) -> Result<OutputStream, ShellError> {
    let cmd_args = BoolArgs {
        bias: args.get_flag("bias")?,
    };

    let mut probability = 0.5;

    if let Some(prob) = cmd_args.bias {
        probability = *prob as f64;

        let probability_is_valid = (0.0..=1.0).contains(&probability);

        if !probability_is_valid {
            return Err(ShellError::labeled_error(
                "The probability is invalid",
                "invalid probability",
                prob.span(),
            ));
        }
    }

    let mut rng = thread_rng();
    let bool_result: bool = rng.gen_bool(probability);

    Ok(OutputStream::one(UntaggedValue::boolean(bool_result)))
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
