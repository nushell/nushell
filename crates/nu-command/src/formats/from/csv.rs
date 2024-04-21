use super::delimited::{from_delimited_data, trim_from_str, DelimitedReaderConfig};
use nu_engine::command_prelude::*;

#[derive(Clone)]
pub struct FromCsv;

impl Command for FromCsv {
    fn name(&self) -> &str {
        "from csv"
    }

    fn signature(&self) -> Signature {
        Signature::build("from csv")
            .input_output_types(vec![(Type::String, Type::table())])
            .named(
                "separator",
                SyntaxShape::String,
                "a character to separate columns (either single char or 4 byte unicode sequence), defaults to ','",
                Some('s'),
            )
            .named(
                "comment",
                SyntaxShape::String,
                "a comment character to ignore lines starting with it",
                Some('c'),
            )
            .named(
                "quote",
                SyntaxShape::String,
                "a quote character to ignore separators in strings, defaults to '\"'",
                Some('q'),
            )
            .named(
                "escape",
                SyntaxShape::String,
                "an escape character for strings containing the quote character",
                Some('e'),
            )
            .switch(
                "noheaders",
                "don't treat the first row as column names",
                Some('n'),
            )
            .switch(
                "flexible",
                "allow the number of fields in records to be variable",
                None,
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
    ) -> Result<PipelineData, ShellError> {
        from_csv(engine_state, stack, call, input)
    }

    fn examples(&self) -> Vec<Example> {
        vec![
            Example {
                description: "Convert comma-separated data to a table",
                example: "\"ColA,ColB\n1,2\" | from csv",
                result: Some(Value::test_list (
                    vec![Value::test_record(record! {
                        "ColA" => Value::test_int(1),
                        "ColB" => Value::test_int(2),
                    })],
                ))
            },
            Example {
                description: "Convert comma-separated data to a table, ignoring headers",
                example: "open data.txt | from csv --noheaders",
                result: None,
            },
            Example {
                description: "Convert semicolon-separated data to a table",
                example: "open data.txt | from csv --separator ';'",
                result: None,
            },
            Example {
                description: "Convert comma-separated data to a table, ignoring lines starting with '#'",
                example: "open data.txt | from csv --comment '#'",
                result: None,
            },
            Example {
                description: "Convert comma-separated data to a table, dropping all possible whitespaces around header names and field values",
                example: "open data.txt | from csv --trim all",
                result: None,
            },
            Example {
                description: "Convert comma-separated data to a table, dropping all possible whitespaces around header names",
                example: "open data.txt | from csv --trim headers",
                result: None,
            },
            Example {
                description: "Convert comma-separated data to a table, dropping all possible whitespaces around field values",
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
    if let PipelineData::Value(Value::List { .. }, _) = input {
        return Err(ShellError::TypeMismatch {
            err_message: "received list stream, did you forget to open file with --raw flag?"
                .into(),
            span: name,
        });
    }

    let separator = match call.get_flag::<String>(engine_state, stack, "separator")? {
        Some(sep) => {
            if sep.len() == 1 {
                sep.chars().next().unwrap_or(',')
            } else if sep.len() == 4 {
                let unicode_sep = u32::from_str_radix(&sep, 16);
                char::from_u32(unicode_sep.unwrap_or(b'\x1f' as u32)).unwrap_or(',')
            } else {
                return Err(ShellError::NonUtf8Custom {
                    msg: "separator should be a single char or a 4-byte unicode".into(),
                    span: call.span(),
                });
            }
        }
        None => ',',
    };
    let comment = call
        .get_flag(engine_state, stack, "comment")?
        .map(|v: Value| v.as_char())
        .transpose()?;
    let quote = call
        .get_flag(engine_state, stack, "quote")?
        .map(|v: Value| v.as_char())
        .transpose()?
        .unwrap_or('"');
    let escape = call
        .get_flag(engine_state, stack, "escape")?
        .map(|v: Value| v.as_char())
        .transpose()?;
    let no_infer = call.has_flag(engine_state, stack, "no-infer")?;
    let noheaders = call.has_flag(engine_state, stack, "noheaders")?;
    let flexible = call.has_flag(engine_state, stack, "flexible")?;
    let trim = trim_from_str(call.get_flag(engine_state, stack, "trim")?)?;

    let config = DelimitedReaderConfig {
        separator,
        comment,
        quote,
        escape,
        noheaders,
        flexible,
        no_infer,
        trim,
    };

    from_delimited_data(config, input, name)
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
