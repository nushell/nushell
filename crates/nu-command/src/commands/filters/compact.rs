use crate::prelude::*;

use nu_engine::WholeStreamCommand;
use nu_errors::ShellError;
use nu_protocol::{Signature, SyntaxShape, UntaggedValue, Value};
use nu_source::Tagged;

pub struct Compact;

pub struct CompactArgs {
    columns: Vec<Tagged<String>>,
}

impl WholeStreamCommand for Compact {
    fn name(&self) -> &str {
        "compact"
    }

    fn signature(&self) -> Signature {
        Signature::build("compact").rest(
            "rest",
            SyntaxShape::Any,
            "the columns to compact from the table",
        )
    }

    fn usage(&self) -> &str {
        "Creates a table with non-empty rows."
    }

    fn run(&self, args: CommandArgs) -> Result<OutputStream, ShellError> {
        compact(args)
    }

    fn examples(&self) -> Vec<Example> {
        vec![Example {
            description: "Filter out all directory entries having no 'target'",
            example: "ls -la | compact target",
            result: None,
        }]
    }
}

pub fn compact(args: CommandArgs) -> Result<OutputStream, ShellError> {
    let (args, input) = (
        CompactArgs {
            columns: args.rest(0)?,
        },
        args.input,
    );

    Ok(input
        .filter(move |item| {
            if args.columns.is_empty() {
                !item.is_empty()
            } else if let Value {
                value: UntaggedValue::Row(ref r),
                ..
            } = item
            {
                args.columns
                    .iter()
                    .all(|field| r.get_data(field).borrow().is_some())
            } else {
                false
            }
        })
        .into_output_stream())
}

#[cfg(test)]
mod tests {
    use super::Compact;
    use super::ShellError;

    #[test]
    fn examples_work_as_expected() -> Result<(), ShellError> {
        use crate::examples::test as test_examples;

        test_examples(Compact {})
    }
}
