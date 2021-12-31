use super::delimited::from_delimited_data;

use nu_protocol::ast::Call;
use nu_protocol::engine::{Command, EngineState, Stack};
use nu_protocol::{Category, Config, PipelineData, ShellError, Signature};

#[derive(Clone)]
pub struct FromTsv;

impl Command for FromTsv {
    fn name(&self) -> &str {
        "from tsv"
    }

    fn signature(&self) -> Signature {
        Signature::build("from tsv")
            .switch(
                "noheaders",
                "don't treat the first row as column names",
                Some('n'),
            )
            .category(Category::Formats)
    }

    fn usage(&self) -> &str {
        "Parse text as .tsv and create table."
    }

    fn run(
        &self,
        _engine_state: &EngineState,
        stack: &mut Stack,
        call: &Call,
        input: PipelineData,
    ) -> Result<nu_protocol::PipelineData, ShellError> {
        let config = stack.get_config().unwrap_or_default();
        from_tsv(call, input, &config)
    }
}

fn from_tsv(call: &Call, input: PipelineData, config: &Config) -> Result<PipelineData, ShellError> {
    let name = call.head;

    let noheaders = call.has_flag("noheaders");

    from_delimited_data(noheaders, '\t', input, name, config)
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test_examples() {
        use crate::test_examples;

        test_examples(FromTsv {})
    }
}
