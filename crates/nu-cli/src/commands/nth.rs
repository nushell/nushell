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
        nth(args, registry)
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

fn nth(args: CommandArgs, registry: &CommandRegistry) -> Result<OutputStream, ShellError> {
    let registry = registry.clone();
    let stream = async_stream! {
        let (NthArgs { row_number, rest: and_rows}, input) = args.process(&registry).await?;

        let mut inp = input.enumerate();
        while let Some((idx, item)) = inp.next().await {
            let row_number = vec![row_number.clone()];

            let row_numbers = vec![&row_number, &and_rows]
                .into_iter()
                .flatten()
                .collect::<Vec<&Tagged<u64>>>();

            if row_numbers
                .iter()
                .any(|requested| requested.item == idx as u64)
            {
                yield ReturnSuccess::value(item);
            }
        }
    };

    Ok(stream.to_output_stream())
}
