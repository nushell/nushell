use crate::prelude::*;
use nu_engine::WholeStreamCommand;
use nu_errors::ShellError;
use nu_protocol::{Signature, SyntaxShape, UntaggedValue, Value};
use nu_source::Tagged;

use super::support::{rotate, Direction};

pub struct Command;

pub struct Arguments {
    by: Option<Tagged<u64>>,
}

impl WholeStreamCommand for Command {
    fn name(&self) -> &str {
        "roll"
    }

    fn signature(&self) -> Signature {
        Signature::build("roll").optional("by", SyntaxShape::Int, "the number of times to roll")
    }

    fn usage(&self) -> &str {
        "Rolls the table rows."
    }

    fn run(&self, args: CommandArgs) -> Result<OutputStream, ShellError> {
        roll(args)
    }
}

pub fn roll(mut args: CommandArgs) -> Result<OutputStream, ShellError> {
    let name = args.call_info.name_tag.clone();

    let options = Arguments { by: args.opt(0)? };

    let values = args.input.drain_vec();

    Ok(roll_down(values, &options)
        .unwrap_or_else(|| vec![UntaggedValue::nothing().into_value(&name)])
        .into_iter()
        .into_output_stream())
}

fn roll_down(values: Vec<Value>, Arguments { by: ref n }: &Arguments) -> Option<Vec<Value>> {
    rotate(values, n, Direction::Down)
}
