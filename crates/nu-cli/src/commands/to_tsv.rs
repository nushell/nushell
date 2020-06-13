use crate::commands::to_delimited_data::to_delimited_data;
use crate::commands::WholeStreamCommand;
use crate::prelude::*;
use nu_errors::ShellError;
use nu_protocol::Signature;

pub struct ToTSV;

#[derive(Deserialize)]
pub struct ToTSVArgs {
    headerless: bool,
}

#[async_trait]
impl WholeStreamCommand for ToTSV {
    fn name(&self) -> &str {
        "to tsv"
    }

    fn signature(&self) -> Signature {
        Signature::build("to tsv").switch(
            "headerless",
            "do not output the column names as the first row",
            None,
        )
    }

    fn usage(&self) -> &str {
        "Convert table into .tsv text"
    }

    async fn run(
        &self,
        args: CommandArgs,
        registry: &CommandRegistry,
    ) -> Result<OutputStream, ShellError> {
        to_tsv(args, registry).await
    }
}

async fn to_tsv(args: CommandArgs, registry: &CommandRegistry) -> Result<OutputStream, ShellError> {
    let registry = registry.clone();
    let name = args.call_info.name_tag.clone();
    let (ToTSVArgs { headerless }, input) = args.process(&registry).await?;

    to_delimited_data(headerless, '\t', "TSV", input, name).await
}

#[cfg(test)]
mod tests {
    use super::ToTSV;

    #[test]
    fn examples_work_as_expected() {
        use crate::examples::test as test_examples;

        test_examples(ToTSV {})
    }
}
