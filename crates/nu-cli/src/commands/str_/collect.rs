use crate::commands::WholeStreamCommand;
use crate::prelude::*;
use nu_errors::ShellError;
use nu_protocol::{ReturnSuccess, Signature, SyntaxShape, UntaggedValue, Value};
use nu_source::Tagged;

pub struct SubCommand;

#[derive(Deserialize)]
pub struct SubCommandArgs {
    separator: Option<Tagged<String>>,
}

#[async_trait]
impl WholeStreamCommand for SubCommand {
    fn name(&self) -> &str {
        "str collect"
    }

    fn signature(&self) -> Signature {
        Signature::build("str collect").desc(self.usage()).optional(
            "separator",
            SyntaxShape::String,
            "the separator to put between the different values",
        )
    }

    fn usage(&self) -> &str {
        "collects a list of strings into a string"
    }

    async fn run(
        &self,
        args: CommandArgs,
        registry: &CommandRegistry,
    ) -> Result<OutputStream, ShellError> {
        collect(args, registry).await
    }

    fn examples(&self) -> Vec<Example> {
        vec![Example {
            description: "Collect a list of string",
            example: "echo ['a' 'b' 'c'] | str collect",
            result: Some(vec![Value::from("abc")]),
        }]
    }
}

pub async fn collect(
    args: CommandArgs,
    registry: &CommandRegistry,
) -> Result<OutputStream, ShellError> {
    let tag = args.call_info.name_tag.clone();
    let (SubCommandArgs { separator }, input) = args.process(registry).await?;
    let separator = separator.map(|tagged| tagged.item).unwrap_or_default();

    let strings: Vec<Result<String, ShellError>> =
        input.map(|value| value.as_string()).collect().await;
    let strings: Vec<String> = strings.into_iter().collect::<Result<_, _>>()?;
    let output = strings.join(&separator);

    Ok(OutputStream::one(ReturnSuccess::value(
        UntaggedValue::string(output).into_value(tag),
    )))
}

#[cfg(test)]
mod tests {
    use super::SubCommand;

    #[test]
    fn examples_work_as_expected() {
        use crate::examples::test as test_examples;

        test_examples(SubCommand {})
    }
}
