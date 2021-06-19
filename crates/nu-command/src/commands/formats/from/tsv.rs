use super::delimited::from_delimited_data;
use crate::prelude::*;
use nu_engine::WholeStreamCommand;
use nu_errors::ShellError;
use nu_protocol::Signature;

pub struct FromTsv;

impl WholeStreamCommand for FromTsv {
    fn name(&self) -> &str {
        "from tsv"
    }

    fn signature(&self) -> Signature {
        Signature::build("from tsv").switch(
            "noheaders",
            "don't treat the first row as column names",
            Some('n'),
        )
    }

    fn usage(&self) -> &str {
        "Parse text as .tsv and create table."
    }

    fn run(&self, args: CommandArgs) -> Result<OutputStream, ShellError> {
        from_tsv(args)
    }
}

fn from_tsv(args: CommandArgs) -> Result<OutputStream, ShellError> {
    let name = args.call_info.name_tag.clone();
    let noheaders = args.has_flag("noheaders");
    let input = args.input;

    from_delimited_data(noheaders, '\t', "TSV", input, name)
}

#[cfg(test)]
mod tests {
    use super::FromTsv;
    use super::ShellError;

    #[test]
    fn examples_work_as_expected() -> Result<(), ShellError> {
        use crate::examples::test as test_examples;

        test_examples(FromTsv {})
    }
}
