use crate::prelude::*;
use nu_engine::WholeStreamCommand;
use nu_errors::ShellError;
use nu_protocol::{ReturnSuccess, Signature, SyntaxShape, UntaggedValue, Value};
use nu_source::Tagged;

use super::support::{rotate, Direction};

pub struct SubCommand;

#[derive(Deserialize)]
pub struct Arguments {
    by: Option<Tagged<u64>>,
}

impl WholeStreamCommand for SubCommand {
    fn name(&self) -> &str {
        "roll up"
    }

    fn signature(&self) -> Signature {
        Signature::build("roll up").optional("by", SyntaxShape::Int, "the number of times to roll")
    }

    fn usage(&self) -> &str {
        "Rolls the table rows"
    }

    fn run_with_actions(&self, args: CommandArgs) -> Result<ActionStream, ShellError> {
        roll(args)
    }
}

pub fn roll(args: CommandArgs) -> Result<ActionStream, ShellError> {
    let name = args.call_info.name_tag.clone();
    let (args, mut input) = args.process()?;

    let values = input.drain_vec();

    Ok((roll_up(values, &args)
        .unwrap_or_else(|| vec![UntaggedValue::nothing().into_value(&name)])
        .into_iter()
        .map(ReturnSuccess::value))
    .to_action_stream())
}

fn roll_up(values: Vec<Value>, Arguments { by: ref n }: &Arguments) -> Option<Vec<Value>> {
    rotate(values, n, Direction::Up)
}
