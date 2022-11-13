use super::delimited::{from_delimited_data, trim_from_str};

use nu_engine::CallExt;
use nu_protocol::ast::Call;
use nu_protocol::engine::{Command, EngineState, Stack};
use nu_protocol::{
    Category, Example, PipelineData, ShellError, Signature, Span, SyntaxShape, Type, Value,
};

#[derive(Clone)]
pub struct FromCsv;

impl Command for FromCsv {
    fn name(&self) -> &str {
        "from csv"
    }

    fn signature(&self) -> Signature {
        Signature::build("from csv")
            .input_output_types(vec![(Type::String, Type::Table(vec![]))])
            .named(
                "separator",
                SyntaxShape::String,
                "a character to separate columns, defaults to ','",
                Some('s'),
            )
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
        "Parse text as .csv and create table."
    }

    fn run(
        &self,
        engine_state: &EngineState,
        stack: &mut Stack,
        call: &Call,
        input: PipelineData,
    ) -> Result<nu_protocol::PipelineData, ShellError> {
        from_csv(engine_state, stack, call, input)
    }

    fn examples(&self) -> Vec<Example> {
        vec![
            Example {
                description: "Convert comma-separated data to a table",
                example: "\"ColA,ColB\n1,2\" | from csv",
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
                description: "Convert comma-separated data to a table, ignoring headers",
                example: "open data.txt | from csv --noheaders",
                result: None,
            },
            Example {
                description: "Convert comma-separated data to a table, ignoring headers",
                example: "open data.txt | from csv -n",
                result: None,
            },
            Example {
                description: "Convert semicolon-separated data to a table",
                example: "open data.txt | from csv --separator ';'",
                result: None,
            },
            Example {
                description: "Convert semicolon-separated data to a table, dropping all possible whitespaces around header names and field values",
                example: "open data.txt | from csv --trim all",
                result: None,
            },
            Example {
                description: "Convert semicolon-separated data to a table, dropping all possible whitespaces around header names",
                example: "open data.txt | from csv --trim headers",
                result: None,
            },
            Example {
                description: "Convert semicolon-separated data to a table, dropping all possible whitespaces around field values",
                example: "open data.txt | from csv --trim fields",
                result: None,
            },
        ]
    }
}

fn from_csv(
    engine_state: &EngineState,
    stack: &mut Stack,
    call: &Call,
    input: PipelineData,
) -> Result<PipelineData, ShellError> {
    let name = call.head;

    let no_infer = call.has_flag("no-infer");
    let noheaders = call.has_flag("noheaders");
    let separator: Option<Value> = call.get_flag(engine_state, stack, "separator")?;
    let trim: Option<Value> = call.get_flag(engine_state, stack, "trim")?;
    let config = engine_state.get_config();

    let sep = match separator {
        Some(Value::String { val: s, span }) => {
            if s == r"\t" {
                '\t'
            } else {
                let vec_s: Vec<char> = s.chars().collect();
                if vec_s.len() != 1 {
                    return Err(ShellError::MissingParameter(
                        "single character separator".into(),
                        span,
                    ));
                };
                vec_s[0]
            }
        }
        _ => ',',
    };

    let trim = trim_from_str(trim)?;

    from_delimited_data(noheaders, no_infer, sep, trim, input, name, config)
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test_examples() {
        use crate::test_examples;

        test_examples(FromCsv {})
    }
}
