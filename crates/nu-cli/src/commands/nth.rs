use crate::commands::WholeStreamCommand;
use crate::context::CommandRegistry;
use crate::prelude::*;
use nu_errors::ShellError;
use nu_protocol::{ReturnSuccess, Signature, SyntaxShape, Value};
use nu_source::Tagged;

#[derive(Deserialize)]
struct NthArgs {
    row_number: Tagged<u64>,
    rest: Vec<Tagged<u64>>,
}

pub struct Nth;

#[async_trait]
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

    async fn run(
        &self,
        args: CommandArgs,
        registry: &CommandRegistry,
    ) -> Result<OutputStream, ShellError> {
        nth(args, registry).await
    }

    fn examples(&self) -> Vec<Example> {
        vec![
            Example {
                description: "Get the second row",
                example: "echo [first second third] | nth 1",
                result: Some(vec![Value::from("second")]),
            },
            Example {
                description: "Get the first and third rows",
                example: "echo [first second third] | nth 0 2",
                result: Some(vec![Value::from("first"), Value::from("third")]),
            },
        ]
    }
}

async fn nth(args: CommandArgs, registry: &CommandRegistry) -> Result<OutputStream, ShellError> {
    let registry = registry.clone();
    let (
        NthArgs {
            row_number,
            rest: and_rows,
        },
        input,
    ) = args.process(&registry).await?;

    let row_numbers = vec![vec![row_number], and_rows]
        .into_iter()
        .flatten()
        .map(|x| x.item)
        .collect::<Vec<u64>>();

    let max_row_number = row_numbers
        .iter()
        .max()
        .expect("Internal error: should be > 0 row numbers");

    Ok(input
        .take(*max_row_number as usize + 1)
        .enumerate()
        .filter_map(move |(idx, item)| {
            futures::future::ready(
                if row_numbers.iter().any(|requested| *requested == idx as u64) {
                    Some(ReturnSuccess::value(item))
                } else {
                    None
                },
            )
        })
        .to_output_stream())
}

#[cfg(test)]
mod tests {
    use super::Nth;

    #[test]
    fn examples_work_as_expected() {
        use crate::examples::test as test_examples;

        test_examples(Nth {})
    }
}
