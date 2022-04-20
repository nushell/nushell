use super::delimited::from_delimited_data;

use nu_protocol::ast::Call;
use nu_protocol::engine::{Command, EngineState, Stack};
use nu_protocol::{Category, Config, Example, PipelineData, ShellError, Signature};

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
        engine_state: &EngineState,
        _stack: &mut Stack,
        call: &Call,
        input: PipelineData,
    ) -> Result<nu_protocol::PipelineData, ShellError> {
        let config = engine_state.get_config();
        from_tsv(call, input, config)
    }

    fn examples(&self) -> Vec<Example> {
        vec![
            Example {
                description: "Create a tsv file with header columns and open it",
                example: r#"echo $'c1(char tab)c2(char tab)c3(char nl)1(char tab)2(char tab)3' | save tsv-data | open tsv-data | from tsv"#,
                result: None,
            },
            Example {
                description: "Create a tsv file without header columns and open it",
                example: r#"echo $'a1(char tab)b1(char tab)c1(char nl)a2(char tab)b2(char tab)c2' | save tsv-data | open tsv-data | from tsv -n"#,
                result: None,
            },
        ]
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
