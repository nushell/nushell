use crate::prelude::*;
use indexmap::{indexmap, IndexMap};
use nu_engine::WholeStreamCommand;
use nu_errors::ShellError;
use nu_protocol::{ReturnSuccess, Signature, SyntaxShape, UntaggedValue, Value};
use nu_source::Tagged;

const DEFAULT_COLUMN_NAME: &str = "Column";

pub struct Wrap;

#[derive(Deserialize)]
struct WrapArgs {
    column: Option<Tagged<String>>,
}

#[async_trait]
impl WholeStreamCommand for Wrap {
    fn name(&self) -> &str {
        "wrap"
    }

    fn signature(&self) -> Signature {
        Signature::build("wrap").optional(
            "column",
            SyntaxShape::String,
            "the name of the new column",
        )
    }

    fn usage(&self) -> &str {
        "Wraps the given data in a table."
    }

    async fn run(&self, args: CommandArgs) -> Result<OutputStream, ShellError> {
        wrap(args).await
    }

    fn examples(&self) -> Vec<Example> {
        vec![
            Example {
                description: "Wrap a list into a table with the default column name",
                example: "echo [1 2 3] | wrap",
                result: Some(vec![
                    UntaggedValue::row(indexmap! {
                        DEFAULT_COLUMN_NAME.to_string() => UntaggedValue::int(1).into(),
                    })
                    .into(),
                    UntaggedValue::row(indexmap! {
                        DEFAULT_COLUMN_NAME.to_string() => UntaggedValue::int(2).into(),
                    })
                    .into(),
                    UntaggedValue::row(indexmap! {
                        DEFAULT_COLUMN_NAME.to_string() => UntaggedValue::int(3).into(),
                    })
                    .into(),
                ]),
            },
            Example {
                description: "Wrap a list into a table with a given column name",
                example: "echo [1 2 3] | wrap MyColumn",
                result: Some(vec![
                    UntaggedValue::row(indexmap! {
                        "MyColumn".to_string() => UntaggedValue::int(1).into(),
                    })
                    .into(),
                    UntaggedValue::row(indexmap! {
                        "MyColumn".to_string() => UntaggedValue::int(2).into(),
                    })
                    .into(),
                    UntaggedValue::row(indexmap! {
                        "MyColumn".to_string() => UntaggedValue::int(3).into(),
                    })
                    .into(),
                ]),
            },
        ]
    }
}

async fn wrap(args: CommandArgs) -> Result<OutputStream, ShellError> {
    let (WrapArgs { column }, mut input) = args.process().await?;
    let mut result_table = vec![];
    let mut are_all_rows = true;

    while let Some(value) = input.next().await {
        match value {
            Value {
                value: UntaggedValue::Row(_),
                ..
            } => {
                result_table.push(value);
            }
            _ => {
                are_all_rows = false;

                let mut index_map = IndexMap::new();
                index_map.insert(
                    match &column {
                        Some(key) => key.item.clone(),
                        None => DEFAULT_COLUMN_NAME.to_string(),
                    },
                    value,
                );

                result_table.push(UntaggedValue::row(index_map).into_value(Tag::unknown()));
            }
        }
    }

    if are_all_rows {
        let mut index_map = IndexMap::new();
        index_map.insert(
            match &column {
                Some(key) => key.item.clone(),
                None => DEFAULT_COLUMN_NAME.to_string(),
            },
            UntaggedValue::table(&result_table).into_value(Tag::unknown()),
        );

        let row = UntaggedValue::row(index_map).into_untagged_value();

        Ok(OutputStream::one(ReturnSuccess::value(row)))
    } else {
        Ok(
            futures::stream::iter(result_table.into_iter().map(ReturnSuccess::value))
                .to_output_stream(),
        )
    }
}

#[cfg(test)]
mod tests {
    use super::ShellError;
    use super::Wrap;

    #[test]
    fn examples_work_as_expected() -> Result<(), ShellError> {
        use crate::examples::test as test_examples;

        test_examples(Wrap {})
    }
}
