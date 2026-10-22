use std::sync::Arc;

use crate::formats::to::delimited::to_delimited_data;
use nu_engine::command_prelude::*;
use nu_protocol::Config;

use super::delimited::ToDelimitedDataArgs;

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
                "A character to separate columns, defaults to ','.",
                Some('s'),
            )
            .named(
                "quote",
                SyntaxShape::String,
                "A quote character to wrap fields containing the separator, defaults to '\"'.",
                Some('q'),
            )
            .named(
                "escape",
                SyntaxShape::String,
                "An escape character for fields containing the quote character, instead of the default doubled-quote escaping.",
                Some('e'),
            )
            .switch(
                "noheaders",
                "Do not output the columns names as the first row.",
                Some('n'),
            )
            .named(
                "columns",
                SyntaxShape::List(SyntaxShape::String.into()),
                "The names (in order) of the columns to use.",
                None,
            )
            .category(Category::Formats)
    }

    fn examples(&self) -> Vec<Example<'_>> {
        vec![
            Example {
                description: "Outputs a CSV string representing the contents of this table.",
                example: "[[foo bar]; [1 2]] | to csv",
                result: Some(Value::test_string("foo,bar\n1,2\n")),
            },
            Example {
                description: "Outputs a CSV string representing the contents of this table.",
                example: "[[foo bar]; [1 2]] | to csv --separator ';' ",
                result: Some(Value::test_string("foo;bar\n1;2\n")),
            },
            Example {
                description: "Outputs a CSV string representing the contents of this record.",
                example: "{a: 1 b: 2} | to csv",
                result: Some(Value::test_string("a,b\n1,2\n")),
            },
            Example {
                description: "Outputs a CSV stream with column names pre-determined.",
                example: "[[foo bar baz]; [1 2 3]] | to csv --columns [baz foo]",
                result: Some(Value::test_string("baz,foo\n3,1\n")),
            },
            Example {
                description: "Outputs a CSV string, quoting fields with a custom character.",
                example: "[[foo]; ['a,b']] | to csv --quote '|'",
                result: Some(Value::test_string("foo\n|a,b|\n")),
            },
            Example {
                description: "Outputs a CSV string, escaping quotes in fields with a custom character instead of doubling them.",
                example: r#"[[foo]; ['say "hi"']] | to csv --escape '\'"#,
                result: Some(Value::test_string("foo\n\"say \\\"hi\\\"\"\n")),
            },
        ]
    }

    fn description(&self) -> &str {
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
        let quote = call
            .get_flag(engine_state, stack, "quote")?
            .map(|v: Value| v.as_char())
            .transpose()?
            .unwrap_or('"');
        let escape = call
            .get_flag(engine_state, stack, "escape")?
            .map(|v: Value| v.as_char())
            .transpose()?;
        let columns: Option<Vec<String>> = call.get_flag(engine_state, stack, "columns")?;
        let config = engine_state.config.clone();
        to_csv(
            input,
            noheaders,
            separator,
            (quote, escape),
            columns,
            head,
            config,
        )
    }
}

fn to_csv(
    input: PipelineData,
    noheaders: bool,
    separator: Option<Spanned<String>>,
    (quote, escape): (char, Option<char>),
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

    to_delimited_data(
        ToDelimitedDataArgs {
            noheaders,
            separator: sep,
            quote,
            escape,
            columns,
            format_name: "CSV",
            input,
            head,
            content_type: Some(mime::TEXT_CSV.to_string()),
        },
        config,
    )
}

#[cfg(test)]
mod test {

    use nu_cmd_lang::eval_pipeline_without_terminal_expression;

    use crate::{Get, Metadata};

    use super::*;

    #[test]
    fn test_examples() -> nu_test_support::Result {
        nu_test_support::test().examples(ToCsv)
    }

    #[test]
    fn test_content_type_metadata() {
        let mut engine_state = Box::new(EngineState::new());
        let delta = {
            // Base functions that are needed for testing
            // Try to keep this working set small to keep tests running as fast as possible
            let mut working_set = StateWorkingSet::new(&engine_state);

            working_set.add_decl(Box::new(ToCsv {}));
            working_set.add_decl(Box::new(Metadata {}));
            working_set.add_decl(Box::new(Get {}));

            working_set.render()
        };

        engine_state
            .merge_delta(delta)
            .expect("Error merging delta");

        let cmd = "{a: 1 b: 2} | to csv  | metadata | get content_type | $in";
        let result = eval_pipeline_without_terminal_expression(
            cmd,
            std::env::temp_dir().as_ref(),
            &mut engine_state,
        );
        assert_eq!(
            Value::test_string("text/csv"),
            result.expect("There should be a result")
        );
    }
}
