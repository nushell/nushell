use crate::prelude::*;
use nu_engine::WholeStreamCommand;
use nu_errors::ShellError;
use nu_protocol::{ReturnSuccess, Signature, SyntaxShape, Value};
use nu_source::Tagged;

#[derive(Deserialize)]
struct NthArgs {
    row_number: Tagged<u64>,
    rest: Vec<Tagged<u64>>,
    skip: bool,
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
            .switch("skip", "Skip the rows instead of selecting them", Some('s'))
    }

    fn usage(&self) -> &str {
        "Return or skip only the selected rows"
    }

    async fn run(&self, args: CommandArgs) -> Result<OutputStream, ShellError> {
        nth(args).await
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
            Example {
                description: "Skip the first and third rows",
                example: "echo [first second third] | nth --skip 0 2",
                result: Some(vec![Value::from("second")]),
            },
        ]
    }
}

async fn nth(args: CommandArgs) -> Result<OutputStream, ShellError> {
    let (
        NthArgs {
            row_number,
            rest: and_rows,
            skip,
        },
        input,
    ) = args.process().await?;

    let row_numbers = vec![vec![row_number], and_rows]
        .into_iter()
        .flatten()
        .map(|x| x.item)
        .collect::<Vec<u64>>();

    Ok(input
        .enumerate()
        .filter_map(move |(idx, item)| {
            futures::future::ready(if row_numbers.contains(&(idx as u64)) ^ skip {
                Some(ReturnSuccess::value(item))
            } else {
                None
            })
        })
        .to_output_stream())
}

#[cfg(test)]
mod tests {
    use super::Nth;
    use super::ShellError;

    #[test]
    fn examples_work_as_expected() -> Result<(), ShellError> {
        use crate::examples::test as test_examples;

        test_examples(Nth {})
    }
}
