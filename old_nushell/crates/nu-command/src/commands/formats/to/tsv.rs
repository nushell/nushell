use crate::commands::formats::to::delimited::to_delimited_data;
use crate::prelude::*;
use nu_engine::WholeStreamCommand;
use nu_errors::ShellError;
use nu_protocol::Signature;

pub struct ToTsv;

impl WholeStreamCommand for ToTsv {
    fn name(&self) -> &str {
        "to tsv"
    }

    fn signature(&self) -> Signature {
        Signature::build("to tsv").switch(
            "noheaders",
            "do not output the column names as the first row",
            Some('n'),
        )
    }

    fn usage(&self) -> &str {
        "Convert table into .tsv text"
    }

    fn run(&self, args: CommandArgs) -> Result<OutputStream, ShellError> {
        to_tsv(args)
    }
}

fn to_tsv(args: CommandArgs) -> Result<OutputStream, ShellError> {
    let name = args.call_info.name_tag.clone();
    let noheaders = args.has_flag("noheaders");

    to_delimited_data(noheaders, '\t', "TSV", args.input, name)
}

#[cfg(test)]
mod tests {
    use super::ShellError;
    use super::ToTsv;

    #[test]
    fn examples_work_as_expected() -> Result<(), ShellError> {
        use crate::examples::test as test_examples;

        test_examples(ToTsv {})
    }
}
