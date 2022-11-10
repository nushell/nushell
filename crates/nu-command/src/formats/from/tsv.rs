use super::delimited::{from_delimited_data, trim_from_str};

use nu_engine::CallExt;
use nu_protocol::ast::Call;
use nu_protocol::engine::{Command, EngineState, Stack};
use nu_protocol::{
    Category, Example, PipelineData, ShellError, Signature, Span, SyntaxShape, Type, Value,
};

#[derive(Clone)]
pub struct FromTsv;

impl Command for FromTsv {
    fn name(&self) -> &str {
        "from tsv"
    }

    fn signature(&self) -> Signature {
        Signature::build("from tsv")
            .input_output_types(vec![(Type::String, Type::Table(vec![]))])
            .switch(
                "noheaders",
                "don't treat the first row as column names",
                Some('n'),
            )
            .switch("no-infer", "no field type inferencing", None)
            .named(
                "trim",
                SyntaxShape::String,
                "drop leading and trailing whitespaces around headers names and/or field values",
                Some('t'),
            )
            .category(Category::Formats)
    }

    fn usage(&self) -> &str {
        "Parse text as .tsv and create table."
    }

    fn run(
        &self,
        engine_state: &EngineState,
        stack: &mut Stack,
        call: &Call,
        input: PipelineData,
    ) -> Result<nu_protocol::PipelineData, ShellError> {
        from_tsv(engine_state, stack, call, input)
    }

    fn examples(&self) -> Vec<Example> {
        vec![
            Example {
                description: "Convert tab-separated data to a table",
                example: "\"ColA\tColB\n1\t2\" | from tsv",
                result: Some(Value::List {
                    vals: vec![Value::Record {
                        cols: vec!["ColA".to_string(), "ColB".to_string()],
                        vals: vec![
                            Value::test_int(1),
                            Value::test_int(2),
                        ],
                        span: Span::test_data(),
                    }],
                    span: Span::test_data(),
                })
            },
            Example {
                description: "Create a tsv file with header columns and open it",
                example: r#"$'c1(char tab)c2(char tab)c3(char nl)1(char tab)2(char tab)3' | save tsv-data | open tsv-data | from tsv"#,
                result: None,
            },
            Example {
                description: "Create a tsv file without header columns and open it",
                example: r#"$'a1(char tab)b1(char tab)c1(char nl)a2(char tab)b2(char tab)c2' | save tsv-data | open tsv-data | from tsv -n"#,
                result: None,
            },
            Example {
                description: "Create a tsv file without header columns and open it, removing all unnecessary whitespaces",
                example: r#"$'a1(char tab)b1(char tab)c1(char nl)a2(char tab)b2(char tab)c2' | save tsv-data | open tsv-data | from tsv --trim all"#,
                result: None,
            },
            Example {
                description: "Create a tsv file without header columns and open it, removing all unnecessary whitespaces in the header names",
                example: r#"$'a1(char tab)b1(char tab)c1(char nl)a2(char tab)b2(char tab)c2' | save tsv-data | open tsv-data | from tsv --trim headers"#,
                result: None,
            },
            Example {
                description: "Create a tsv file without header columns and open it, removing all unnecessary whitespaces in the field values",
                example: r#"$'a1(char tab)b1(char tab)c1(char nl)a2(char tab)b2(char tab)c2' | save tsv-data | open tsv-data | from tsv --trim fields"#,
                result: None,
            },
        ]
    }
}

fn from_tsv(
    engine_state: &EngineState,
    stack: &mut Stack,
    call: &Call,
    input: PipelineData,
) -> Result<PipelineData, ShellError> {
    let name = call.head;

    let no_infer = call.has_flag("no-infer");
    let noheaders = call.has_flag("noheaders");
    let trim: Option<Value> = call.get_flag(engine_state, stack, "trim")?;
    let trim = trim_from_str(trim)?;

    from_delimited_data(
        noheaders,
        no_infer,
        '\t',
        trim,
        input,
        name,
        engine_state.get_config(),
    )
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
