use crate::prelude::*;
use nu_engine::WholeStreamCommand;
use nu_errors::ShellError;
use nu_protocol::{Signature, SyntaxShape, UntaggedValue};
use nu_source::Tagged;
use rand::prelude::{thread_rng, Rng};

pub struct SubCommand;

pub struct DiceArgs {
    dice: Option<Tagged<u32>>,
    sides: Option<Tagged<u32>>,
}

impl WholeStreamCommand for SubCommand {
    fn name(&self) -> &str {
        "random dice"
    }

    fn signature(&self) -> Signature {
        Signature::build("random dice")
            .named(
                "dice",
                SyntaxShape::Int,
                "The amount of dice being rolled",
                Some('d'),
            )
            .named(
                "sides",
                SyntaxShape::Int,
                "The amount of sides a die has",
                Some('s'),
            )
    }

    fn usage(&self) -> &str {
        "Generate a random dice roll"
    }

    fn run(&self, args: CommandArgs) -> Result<OutputStream, ShellError> {
        dice(args)
    }

    fn examples(&self) -> Vec<Example> {
        vec![
            Example {
                description: "Roll 1 dice with 6 sides each",
                example: "random dice",
                result: None,
            },
            Example {
                description: "Roll 10 dice with 12 sides each",
                example: "random dice -d 10 -s 12",
                result: None,
            },
        ]
    }
}

pub fn dice(args: CommandArgs) -> Result<OutputStream, ShellError> {
    let tag = args.call_info.name_tag.clone();

    let cmd_args = DiceArgs {
        dice: args.get_flag("dice")?,
        sides: args.get_flag("sides")?,
    };

    let dice = if let Some(dice_tagged) = cmd_args.dice {
        *dice_tagged
    } else {
        1
    };

    let sides = if let Some(sides_tagged) = cmd_args.sides {
        *sides_tagged
    } else {
        6
    };

    let iter = (0..dice).map(move |_| {
        let mut thread_rng = thread_rng();
        UntaggedValue::int(thread_rng.gen_range(1..sides + 1)).into_value(tag.clone())
    });

    Ok(iter.into_output_stream())
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
