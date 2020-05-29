use crate::commands::WholeStreamCommand;
use crate::context::CommandRegistry;
use crate::prelude::*;
use nu_errors::ShellError;
use nu_protocol::{ReturnSuccess, Signature, SyntaxShape, UntaggedValue};
use nu_source::Tagged;

pub struct Keep;

#[derive(Deserialize)]
pub struct KeepArgs {
    rows: Option<Tagged<usize>>,
}

#[async_trait]
impl WholeStreamCommand for Keep {
    fn name(&self) -> &str {
        "keep"
    }

    fn signature(&self) -> Signature {
        Signature::build("keep").optional(
            "rows",
            SyntaxShape::Int,
            "starting from the front, the number of rows to keep",
        )
    }

    fn usage(&self) -> &str {
        "Keep the number of rows only"
    }

    async fn run(
        &self,
        args: CommandArgs,
        registry: &CommandRegistry,
    ) -> Result<OutputStream, ShellError> {
        keep(args, registry)
    }

    fn examples(&self) -> Vec<Example> {
        vec![
            Example {
                description: "Keep the first row",
                example: "echo [1 2 3] | keep",
                result: Some(vec![UntaggedValue::int(1).into()]),
            },
            Example {
                description: "Keep the first four rows",
                example: "echo [1 2 3 4 5] | keep 4",
                result: Some(vec![
                    UntaggedValue::int(1).into(),
                    UntaggedValue::int(2).into(),
                    UntaggedValue::int(3).into(),
                    UntaggedValue::int(4).into(),
                ]),
            },
        ]
    }
}

fn keep(args: CommandArgs, registry: &CommandRegistry) -> Result<OutputStream, ShellError> {
    let registry = registry.clone();
    let stream = async_stream! {
        let (KeepArgs { rows }, mut input) = args.process(&registry).await?;
        let mut rows_desired = if let Some(quantity) = rows {
            *quantity
        } else {
            1
        };

        while let Some(input) = input.next().await {
            if rows_desired > 0 {
                yield ReturnSuccess::value(input);
                rows_desired -= 1;
            } else {
                break;
            }
        }
    };

    Ok(stream.to_output_stream())
}

#[cfg(test)]
mod tests {
    use super::Keep;

    #[test]
    fn examples_work_as_expected() {
        use crate::examples::test as test_examples;

        test_examples(Keep {})
    }
}
