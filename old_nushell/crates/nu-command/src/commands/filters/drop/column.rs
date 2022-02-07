use crate::prelude::*;
use nu_data::base::select_fields;
use nu_engine::WholeStreamCommand;
use nu_errors::ShellError;
use nu_protocol::{Signature, SyntaxShape};
use nu_source::Tagged;

pub struct SubCommand;

impl WholeStreamCommand for SubCommand {
    fn name(&self) -> &str {
        "drop column"
    }

    fn signature(&self) -> Signature {
        Signature::build("drop column").optional(
            "columns",
            SyntaxShape::Number,
            "starting from the end, the number of columns to remove",
        )
    }

    fn usage(&self) -> &str {
        "Remove the last number of columns. If you want to remove columns by name, try 'reject'."
    }

    fn run(&self, args: CommandArgs) -> Result<OutputStream, ShellError> {
        drop(args)
    }

    fn examples(&self) -> Vec<Example> {
        use nu_protocol::Value;

        vec![Example {
            description: "Remove the last column of a table",
            example: "echo [[lib, extension]; [nu-lib, rs] [nu-core, rb]] | drop column",
            result: Some(vec![
                row! { "lib".into() => Value::from("nu-lib") },
                row! { "lib".into() => Value::from("nu-core") },
            ]),
        }]
    }
}

fn drop(args: CommandArgs) -> Result<OutputStream, ShellError> {
    let columns: Option<Tagged<u64>> = args.opt(0)?;

    let to_drop = if let Some(quantity) = columns {
        *quantity as usize
    } else {
        1
    };

    Ok(args
        .input
        .map(move |item| {
            let headers = item.data_descriptors();

            let descs = match headers.len() {
                0 => &headers[..],
                n if to_drop > n => &[],
                n => &headers[..n - to_drop],
            };

            Ok(select_fields(&item, descs, item.tag()))
        })
        .into_input_stream())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn examples_work_as_expected() -> Result<(), ShellError> {
        use crate::examples::test as test_examples;

        test_examples(SubCommand {})
    }
}
