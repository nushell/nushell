use crate::commands::WholeStreamCommand;
use crate::context::CommandRegistry;
use crate::prelude::*;
use nu_errors::ShellError;
use nu_protocol::{ReturnSuccess, Signature, SyntaxShape, UntaggedValue, Value};
use nu_source::Tagged;
use nu_value_ext::ValueExt;

#[derive(Deserialize)]
struct DefaultArgs {
    column: Tagged<String>,
    value: Value,
}

pub struct Default;

#[async_trait]
impl WholeStreamCommand for Default {
    fn name(&self) -> &str {
        "default"
    }

    fn signature(&self) -> Signature {
        Signature::build("default")
            .required("column name", SyntaxShape::String, "the name of the column")
            .required(
                "column value",
                SyntaxShape::Any,
                "the value of the column to default",
            )
    }

    fn usage(&self) -> &str {
        "Sets a default row's column if missing."
    }

    async fn run(
        &self,
        args: CommandArgs,
        registry: &CommandRegistry,
    ) -> Result<OutputStream, ShellError> {
        default(args, registry)
    }

    fn examples(&self) -> Vec<Example> {
        vec![Example {
            description: "Give a default 'target' to all file entries",
            example: "ls -af | default target 'nothing'",
            result: None,
        }]
    }
}

fn default(args: CommandArgs, registry: &CommandRegistry) -> Result<OutputStream, ShellError> {
    let registry = registry.clone();
    let stream = async_stream! {
        let (DefaultArgs { column, value }, mut input) = args.process(&registry).await?;
        while let Some(item) = input.next().await {
            let should_add = match item {
                Value {
                    value: UntaggedValue::Row(ref r),
                    ..
                } => r.get_data(&column.item).borrow().is_none(),
                _ => false,
            };

            if should_add {
                match item.insert_data_at_path(&column.item, value.clone()) {
                    Some(new_value) => yield ReturnSuccess::value(new_value),
                    None => yield ReturnSuccess::value(item),
                }
            } else {
                yield ReturnSuccess::value(item);
            }

        }
    };

    Ok(stream.to_output_stream())
}

#[cfg(test)]
mod tests {
    use super::Default;

    #[test]
    fn examples_work_as_expected() {
        use crate::examples::test as test_examples;

        test_examples(Default {})
    }
}
