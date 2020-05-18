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

    fn run(
        &self,
        args: CommandArgs,
        registry: &CommandRegistry,
    ) -> Result<OutputStream, ShellError> {
        reject(args, registry)
    }

    fn examples(&self) -> Vec<Example> {
        vec![Example {
            description: "Lists the files in a directory without showing the modified column",
            example: "ls | reject modified",
            result: None,
        }]
    }
}

fn reject(args: CommandArgs, registry: &CommandRegistry) -> Result<OutputStream, ShellError> {
    let registry = registry.clone();
    let stream = async_stream! {
        let name = args.call_info.name_tag.clone();
        let (RejectArgs { rest: fields }, mut input) = args.process(&registry).await?;
        if fields.is_empty() {
            yield Err(ShellError::labeled_error(
                "Reject requires fields",
                "needs parameter",
                name,
            ));
            return;
        }

        let fields: Vec<_> = fields.iter().map(|f| f.item.clone()).collect();

        while let Some(item) = input.next().await {
            yield ReturnSuccess::value(reject_fields(&item, &fields, &item.tag));
        }
    };

    Ok(stream.to_output_stream())
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
