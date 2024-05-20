use std::sync::Arc;

use crate::formats::to::delimited::to_delimited_data;
use nu_engine::command_prelude::*;
use nu_protocol::Config;

#[derive(Clone)]
pub struct ToCsv;

impl Command for ToCsv {
    fn name(&self) -> &str {
        "to csv"
    }

    fn signature(&self) -> Signature {
        Signature::build("to csv")
            .input_output_types(vec![
                (Type::record(), Type::String),
                (Type::table(), Type::String),
            ])
            .named(
                "separator",
                SyntaxShape::String,
                "a character to separate columns, defaults to ','",
                Some('s'),
            )
            .switch(
                "noheaders",
                "do not output the columns names as the first row",
                Some('n'),
            )
            .named(
                "columns",
                SyntaxShape::List(SyntaxShape::String.into()),
                "the names (in order) of the columns to use",
                None,
            )
            .category(Category::Formats)
    }

    fn examples(&self) -> Vec<Example> {
        vec![
            Example {
                description: "Outputs a CSV string representing the contents of this table",
                example: "[[foo bar]; [1 2]] | to csv",
                result: Some(Value::test_string("foo,bar\n1,2\n")),
            },
            Example {
                description: "Outputs a CSV string representing the contents of this table",
                example: "[[foo bar]; [1 2]] | to csv --separator ';' ",
                result: Some(Value::test_string("foo;bar\n1;2\n")),
            },
            Example {
                description: "Outputs a CSV string representing the contents of this record",
                example: "{a: 1 b: 2} | to csv",
                result: Some(Value::test_string("a,b\n1,2\n")),
            },
            Example {
                description: "Outputs a CSV stream with column names pre-determined",
                example: "[[foo bar baz]; [1 2 3]] | to csv --columns [baz foo]",
                result: Some(Value::test_string("baz,foo\n3,1\n")),
            },
        ]
    }

    fn usage(&self) -> &str {
        "Convert table into .csv text ."
    }

    fn run(
        &self,
        engine_state: &EngineState,
        stack: &mut Stack,
        call: &Call,
        input: PipelineData,
    ) -> Result<PipelineData, ShellError> {
        let head = call.head;
        let noheaders = call.has_flag(engine_state, stack, "noheaders")?;
        let separator: Option<Spanned<String>> = call.get_flag(engine_state, stack, "separator")?;
        let columns: Option<Vec<String>> = call.get_flag(engine_state, stack, "columns")?;
        let config = engine_state.config.clone();
        to_csv(input, noheaders, separator, columns, head, config)
    }
}

fn to_csv(
    input: PipelineData,
    noheaders: bool,
    separator: Option<Spanned<String>>,
    columns: Option<Vec<String>>,
    head: Span,
    config: Arc<Config>,
) -> Result<PipelineData, ShellError> {
    let sep = match separator {
        Some(Spanned { item: s, span, .. }) => {
            if s == r"\t" {
                Spanned { item: '\t', span }
            } else {
                let vec_s: Vec<char> = s.chars().collect();
                if vec_s.len() != 1 {
                    return Err(ShellError::TypeMismatch {
                        err_message: "Expected a single separator char from --separator"
                            .to_string(),
                        span,
                    });
                };
                Spanned {
                    item: vec_s[0],
                    span: head,
                }
            }
        }
        _ => Spanned {
            item: ',',
            span: head,
        },
    };

    to_delimited_data(noheaders, sep, columns, "CSV", input, head, config)
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test_examples() {
        use crate::test_examples;

        test_examples(ToCsv {})
    }
}
