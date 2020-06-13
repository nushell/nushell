use crate::commands::WholeStreamCommand;
use crate::prelude::*;
use indexmap::IndexMap;
use nu_errors::ShellError;
use nu_protocol::{ReturnSuccess, Signature, SyntaxShape, UntaggedValue, Value};
use nu_source::Tagged;

pub struct Rename;

#[derive(Deserialize)]
pub struct Arguments {
    column_name: Tagged<String>,
    rest: Vec<Tagged<String>>,
}

#[async_trait]
impl WholeStreamCommand for Rename {
    fn name(&self) -> &str {
        "rename"
    }

    fn signature(&self) -> Signature {
        Signature::build("rename")
            .required(
                "column_name",
                SyntaxShape::String,
                "the new name for the first column",
            )
            .rest(SyntaxShape::String, "the new name for additional columns")
    }

    fn usage(&self) -> &str {
        "Creates a new table with columns renamed."
    }

    async fn run(
        &self,
        args: CommandArgs,
        registry: &CommandRegistry,
    ) -> Result<OutputStream, ShellError> {
        rename(args, registry).await
    }

    fn examples(&self) -> Vec<Example> {
        vec![
            Example {
                description: "Rename a column",
                example: r#"echo "{a: 1, b: 2, c: 3}" | from json | rename my_column"#,
                result: None,
            },
            Example {
                description: "Rename many columns",
                example: r#"echo "{a: 1, b: 2, c: 3}" | from json | rename spam eggs cars"#,
                result: None,
            },
        ]
    }
}

pub async fn rename(
    args: CommandArgs,
    registry: &CommandRegistry,
) -> Result<OutputStream, ShellError> {
    let registry = registry.clone();
    let name = args.call_info.name_tag.clone();
    let (Arguments { column_name, rest }, input) = args.process(&registry).await?;
    let mut new_column_names = vec![vec![column_name]];
    new_column_names.push(rest);

    let new_column_names = new_column_names.into_iter().flatten().collect::<Vec<_>>();

    Ok(input
        .map(move |item| {
            if let Value {
                value: UntaggedValue::Row(row),
                tag,
            } = item
            {
                let mut renamed_row = IndexMap::new();

                for (idx, (key, value)) in row.entries.iter().enumerate() {
                    let key = if idx < new_column_names.len() {
                        &new_column_names[idx].item
                    } else {
                        key
                    };

                    renamed_row.insert(key.clone(), value.clone());
                }

                let out = UntaggedValue::Row(renamed_row.into()).into_value(tag);

                ReturnSuccess::value(out)
            } else {
                ReturnSuccess::value(
                    UntaggedValue::Error(ShellError::labeled_error(
                        "no column names available",
                        "can't rename",
                        &name,
                    ))
                    .into_untagged_value(),
                )
            }
        })
        .to_output_stream())
}

#[cfg(test)]
mod tests {
    use super::Rename;

    #[test]
    fn examples_work_as_expected() {
        use crate::examples::test as test_examples;

        test_examples(Rename {})
    }
}
