use crate::prelude::*;
use nu_engine::WholeStreamCommand;
use nu_errors::ShellError;
use nu_protocol::{Signature, SyntaxShape, UntaggedValue, Value};

#[derive(Deserialize)]
struct PrependArgs {
    row: Value,
}

pub struct Prepend;

impl WholeStreamCommand for Prepend {
    fn name(&self) -> &str {
        "prepend"
    }

    fn signature(&self) -> Signature {
        Signature::build("prepend").required(
            "row value",
            SyntaxShape::Any,
            "the value of the row to prepend to the table",
        )
    }

    fn usage(&self) -> &str {
        "Prepend the given row to the front of the table."
    }

    fn run_with_actions(&self, args: CommandArgs) -> Result<ActionStream, ShellError> {
        prepend(args)
    }

    fn examples(&self) -> Vec<Example> {
        vec![Example {
            description: "Add something to the beginning of a list or table",
            example: "echo [2 3 4] | prepend 1",
            result: Some(vec![
                UntaggedValue::int(1).into(),
                UntaggedValue::int(2).into(),
                UntaggedValue::int(3).into(),
                UntaggedValue::int(4).into(),
            ]),
        }]
    }
}

fn prepend(args: CommandArgs) -> Result<ActionStream, ShellError> {
    let (PrependArgs { row }, input) = args.process()?;

    let bos = vec![row].into_iter();

    Ok(bos.chain(input).to_output_stream_with_actions())
}

#[cfg(test)]
mod tests {
    use super::Prepend;
    use super::ShellError;

    #[test]
    fn examples_work_as_expected() -> Result<(), ShellError> {
        use crate::examples::test as test_examples;

        test_examples(Prepend {})
    }
}
