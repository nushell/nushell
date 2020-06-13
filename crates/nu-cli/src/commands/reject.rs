use crate::commands::WholeStreamCommand;
use crate::data::base::reject_fields;
use crate::prelude::*;
use nu_errors::ShellError;
use nu_protocol::{ReturnSuccess, Signature, SyntaxShape};
use nu_source::Tagged;

#[derive(Deserialize)]
pub struct RejectArgs {
    rest: Vec<Tagged<String>>,
}

pub struct Reject;

#[async_trait]
impl WholeStreamCommand for Reject {
    fn name(&self) -> &str {
        "reject"
    }

    fn signature(&self) -> Signature {
        Signature::build("reject").rest(SyntaxShape::String, "the names of columns to remove")
    }

    fn usage(&self) -> &str {
        "Remove the given columns from the table."
    }

    async fn run(
        &self,
        args: CommandArgs,
        registry: &CommandRegistry,
    ) -> Result<OutputStream, ShellError> {
        reject(args, registry).await
    }

    fn examples(&self) -> Vec<Example> {
        vec![Example {
            description: "Lists the files in a directory without showing the modified column",
            example: "ls | reject modified",
            result: None,
        }]
    }
}

async fn reject(args: CommandArgs, registry: &CommandRegistry) -> Result<OutputStream, ShellError> {
    let registry = registry.clone();
    let name = args.call_info.name_tag.clone();
    let (RejectArgs { rest: fields }, input) = args.process(&registry).await?;
    if fields.is_empty() {
        return Err(ShellError::labeled_error(
            "Reject requires fields",
            "needs parameter",
            name,
        ));
    }

    let fields: Vec<_> = fields.iter().map(|f| f.item.clone()).collect();

    Ok(input
        .map(move |item| ReturnSuccess::value(reject_fields(&item, &fields, &item.tag)))
        .to_output_stream())
}

#[cfg(test)]
mod tests {
    use super::Reject;

    #[test]
    fn examples_work_as_expected() {
        use crate::examples::test as test_examples;

        test_examples(Reject {})
    }
}
