use crate::formats::to::delimited::to_delimited_data;
use nu_protocol::ast::Call;
use nu_protocol::engine::{Command, EngineState, Stack};
use nu_protocol::{
    Category, Config, Example, PipelineData, ShellError, Signature, Span, Type, Value,
};

#[derive(Clone)]
pub struct ToTsv;

impl Command for ToTsv {
    fn name(&self) -> &str {
        "to tsv"
    }

    fn signature(&self) -> Signature {
        Signature::build("to tsv")
            .input_output_types(vec![(Type::Any, Type::String)])
            .switch(
                "noheaders",
                "do not output the column names as the first row",
                Some('n'),
            )
            .category(Category::Formats)
    }

    fn usage(&self) -> &str {
        "Convert table into .tsv text"
    }

    fn examples(&self) -> Vec<Example> {
        vec![Example {
            description: "Outputs an TSV string representing the contents of this table",
            example: "[[foo bar]; [1 2]] | to tsv",
            result: Some(Value::test_string("foo\tbar\n1\t2\n")),
        }]
    }

    fn run(
        &self,
        engine_state: &EngineState,
        _stack: &mut Stack,
        call: &Call,
        input: PipelineData,
    ) -> Result<nu_protocol::PipelineData, ShellError> {
        let head = call.head;
        let noheaders = call.has_flag("noheaders");
        let config = engine_state.get_config();
        to_tsv(input, noheaders, head, config)
    }
}

fn to_tsv(
    input: PipelineData,
    noheaders: bool,
    head: Span,
    config: &Config,
) -> Result<PipelineData, ShellError> {
    to_delimited_data(noheaders, '\t', "TSV", input, head, config)
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test_examples() {
        use crate::test_examples;

        test_examples(ToTsv {})
    }
}
