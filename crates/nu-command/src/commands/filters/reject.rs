use crate::prelude::*;
use nu_data::base::reject_fields;
use nu_engine::WholeStreamCommand;
use nu_errors::ShellError;
use nu_protocol::{ReturnSuccess, Signature, SyntaxShape};
use nu_source::Tagged;

pub struct Reject;

impl WholeStreamCommand for Reject {
    fn name(&self) -> &str {
        "reject"
    }

    fn signature(&self) -> Signature {
        Signature::build("reject").rest(SyntaxShape::String, "the names of columns to remove")
    }

    fn usage(&self) -> &str {
        "Remove the given columns from the table. If you want to remove rows, try 'drop'."
    }

    fn run_with_actions(&self, args: CommandArgs) -> Result<ActionStream, ShellError> {
        reject(args)
    }

    fn examples(&self) -> Vec<Example> {
        vec![Example {
            description: "Lists the files in a directory without showing the modified column",
            example: "ls | reject modified",
            result: None,
        }]
    }
}

fn reject(args: CommandArgs) -> Result<ActionStream, ShellError> {
    let name = args.call_info.name_tag.clone();
    let fields: Vec<Tagged<String>> = args.rest(0)?;

    if fields.is_empty() {
        return Err(ShellError::labeled_error(
            "Reject requires fields",
            "needs parameter",
            name,
        ));
    }

    let fields: Vec<_> = fields.iter().map(|f| f.item.clone()).collect();

    Ok(args
        .input
        .map(move |item| ReturnSuccess::value(reject_fields(&item, &fields, &item.tag)))
        .into_action_stream())
}

#[cfg(test)]
mod tests {
    use super::Reject;
    use super::ShellError;

    #[test]
    fn examples_work_as_expected() -> Result<(), ShellError> {
        use crate::examples::test as test_examples;

        test_examples(Reject {})
    }
}
