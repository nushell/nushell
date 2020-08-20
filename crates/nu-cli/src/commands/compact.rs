use crate::commands::WholeStreamCommand;
use crate::context::CommandRegistry;
use crate::prelude::*;
use futures::future;
use futures::stream::StreamExt;
use nu_errors::ShellError;
use nu_protocol::{ReturnSuccess, Signature, SyntaxShape, UntaggedValue, Value};
use nu_source::Tagged;

pub struct Compact;

#[derive(Deserialize)]
pub struct CompactArgs {
    rest: Vec<Tagged<String>>,
}

#[async_trait]
impl WholeStreamCommand for Compact {
    fn name(&self) -> &str {
        "compact"
    }

    fn signature(&self) -> Signature {
        Signature::build("compact").rest(SyntaxShape::Any, "the columns to compact from the table")
    }

    fn usage(&self) -> &str {
        "Creates a table with non-empty rows"
    }

    async fn run(
        &self,
        args: CommandArgs,
        registry: &CommandRegistry,
    ) -> Result<OutputStream, ShellError> {
        compact(args, registry).await
    }

    fn examples(&self) -> Vec<Example> {
        vec![
            Example {
                description: "Filter out all null entries in a list",
                example: "echo [1 2 $null 3 $null $null] | compact",
                result: Some(vec![
                    UntaggedValue::int(1).into(),
                    UntaggedValue::int(2).into(),
                    UntaggedValue::int(3).into(),
                ]),
            },
            Example {
                description: "Filter out all directory entries having no 'target'",
                example: "ls -la | compact target",
                result: None,
            },
        ]
    }
}

pub async fn compact(
    args: CommandArgs,
    registry: &CommandRegistry,
) -> Result<OutputStream, ShellError> {
    let registry = registry.clone();
    let (CompactArgs { rest: columns }, input) = args.process(&registry).await?;
    Ok(input
        .filter_map(move |item| {
            future::ready(if columns.is_empty() {
                if !item.is_empty() {
                    Some(ReturnSuccess::value(item))
                } else {
                    None
                }
            } else {
                match item {
                    Value {
                        value: UntaggedValue::Row(ref r),
                        ..
                    } => {
                        if columns
                            .iter()
                            .all(|field| r.get_data(field).borrow().is_some())
                        {
                            Some(ReturnSuccess::value(item))
                        } else {
                            None
                        }
                    }
                    _ => None,
                }
            })
        })
        .to_output_stream())
}

#[cfg(test)]
mod tests {
    use super::Compact;

    #[test]
    fn examples_work_as_expected() {
        use crate::examples::test as test_examples;

        test_examples(Compact {})
    }
}
