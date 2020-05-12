use crate::commands::WholeStreamCommand;
use crate::context::CommandRegistry;
use crate::prelude::*;
use nu_errors::ShellError;
use nu_protocol::{ReturnSuccess, Signature, SyntaxShape};
use nu_source::Tagged;

#[derive(Deserialize)]
struct NthArgs {
    row_number: Tagged<u64>,
    rest: Vec<Tagged<u64>>,
}

pub struct Nth;

impl WholeStreamCommand for Nth {
    fn name(&self) -> &str {
        "nth"
    }

    fn signature(&self) -> Signature {
        Signature::build("nth")
            .required(
                "row number",
                SyntaxShape::Int,
                "the number of the row to return",
            )
            .rest(SyntaxShape::Any, "Optionally return more rows")
    }

    fn usage(&self) -> &str {
        "Return only the selected rows"
    }

    fn run(
        &self,
        args: CommandArgs,
        registry: &CommandRegistry,
    ) -> Result<OutputStream, ShellError> {
        args.process(registry, nth)?.run()
    }

    fn examples(&self) -> &[Example] {
        &[
            Example {
                description: "Get the second row",
                example: "echo [first second third] | nth 1",
            },
            Example {
                description: "Get the first and third rows",
                example: "echo [first second third] | nth 0 2",
            },
        ]
    }
}

fn nth(
    NthArgs {
        row_number,
        rest: and_rows,
    }: NthArgs,
    RunnableContext { input, .. }: RunnableContext,
) -> Result<OutputStream, ShellError> {
    let stream = input
        .enumerate()
        .map(move |(idx, item)| {
            let row_number = vec![row_number.clone()];

            let row_numbers = vec![&row_number, &and_rows]
                .into_iter()
                .flatten()
                .collect::<Vec<&Tagged<u64>>>();

            let mut result = VecDeque::new();

            if row_numbers
                .iter()
                .any(|requested| requested.item == idx as u64)
            {
                result.push_back(ReturnSuccess::value(item));
            }

            futures::stream::iter(result)
        })
        .flatten();

    Ok(stream.to_output_stream())
}
