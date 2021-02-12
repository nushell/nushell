use crate::prelude::*;
use nu_engine::WholeStreamCommand;
use nu_errors::ShellError;
use nu_protocol::{Signature, SyntaxShape, UntaggedValue};
use nu_source::Tagged;

pub struct First;

#[derive(Deserialize)]
pub struct FirstArgs {
    rows: Option<Tagged<usize>>,
}

#[async_trait]
impl WholeStreamCommand for First {
    fn name(&self) -> &str {
        "first"
    }

    fn signature(&self) -> Signature {
        Signature::build("first").optional(
            "rows",
            SyntaxShape::Int,
            "starting from the front, the number of rows to return",
        )
    }

    fn usage(&self) -> &str {
        "Show only the first number of rows."
    }

    async fn run(&self, args: CommandArgs) -> Result<OutputStream, ShellError> {
        first(args).await
    }

    fn examples(&self) -> Vec<Example> {
        vec![
            Example {
                description: "Return the first item of a list/table",
                example: "echo [1 2 3] | first",
                result: Some(vec![UntaggedValue::int(1).into()]),
            },
            Example {
                description: "Return the first 2 items of a list/table",
                example: "echo [1 2 3] | first 2",
                result: Some(vec![
                    UntaggedValue::int(1).into(),
                    UntaggedValue::int(2).into(),
                ]),
            },
        ]
    }
}

async fn first(args: CommandArgs) -> Result<OutputStream, ShellError> {
    let (FirstArgs { rows }, input) = args.process().await?;
    let rows_desired = if let Some(quantity) = rows {
        *quantity
    } else {
        1
    };

    Ok(input.take(rows_desired).to_output_stream())
}

#[cfg(test)]
mod tests {
    use super::First;
    use super::ShellError;

    #[test]
    fn examples_work_as_expected() -> Result<(), ShellError> {
        use crate::examples::test as test_examples;

        test_examples(First {})
    }
}
