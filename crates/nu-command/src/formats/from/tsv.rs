use super::delimited::from_delimited_data;

use nu_protocol::ast::Call;
use nu_protocol::engine::{Command, EngineState, Stack};
use nu_protocol::{PipelineData, ShellError, Signature};

#[derive(Clone)]
pub struct FromTsv;

impl Command for FromTsv {
    fn name(&self) -> &str {
        "from tsv"
    }

    fn signature(&self) -> Signature {
        Signature::build("from csv").switch(
            "noheaders",
            "don't treat the first row as column names",
            Some('n'),
        )
    }

    fn usage(&self) -> &str {
        "Parse text as .csv and create table."
    }

    fn run(
        &self,
        _engine_state: &EngineState,
        _stack: &mut Stack,
        call: &Call,
        input: PipelineData,
    ) -> Result<nu_protocol::PipelineData, ShellError> {
        from_tsv(call, input)
    }
}

fn from_tsv(call: &Call, input: PipelineData) -> Result<PipelineData, ShellError> {
    let name = call.head;

    let noheaders = call.has_flag("noheaders");

    from_delimited_data(noheaders, '\t', input, name)
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
