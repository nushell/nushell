use std::sync::Arc;

use crate::formats::to::delimited::to_delimited_data;
use nu_engine::command_prelude::*;
use nu_protocol::Config;

use super::delimited::ToDelimitedDataArgs;

#[derive(Clone)]
pub struct ToTsv;

impl Command for ToTsv {
    fn name(&self) -> &str {
        "to tsv"
    }

    fn signature(&self) -> Signature {
        Signature::build("to tsv")
            .input_output_types(vec![
                (Type::record(), Type::String),
                (Type::table(), Type::String),
            ])
            .switch(
                "noheaders",
                "do not output the column names as the first row",
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

    fn description(&self) -> &str {
        "Convert table into .tsv text."
    }

    fn examples(&self) -> Vec<Example<'_>> {
        vec![
            Example {
                description: "Outputs a TSV string representing the contents of this table",
                example: "[[foo bar]; [1 2]] | to tsv",
                result: Some(Value::test_string("foo\tbar\n1\t2\n")),
            },
            Example {
                description: "Outputs a TSV string representing the contents of this record",
                example: "{a: 1 b: 2} | to tsv",
                result: Some(Value::test_string("a\tb\n1\t2\n")),
            },
            Example {
                description: "Outputs a TSV stream with column names pre-determined",
                example: "[[foo bar baz]; [1 2 3]] | to tsv --columns [baz foo]",
                result: Some(Value::test_string("baz\tfoo\n3\t1\n")),
            },
        ]
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
        let columns: Option<Vec<String>> = call.get_flag(engine_state, stack, "columns")?;
        let config = engine_state.config.clone();
        to_tsv(input, noheaders, columns, head, config)
    }
}

fn to_tsv(
    input: PipelineData,
    noheaders: bool,
    columns: Option<Vec<String>>,
    head: Span,
    config: Arc<Config>,
) -> Result<PipelineData, ShellError> {
    let sep = Spanned {
        item: '\t',
        span: head,
    };
    to_delimited_data(
        ToDelimitedDataArgs {
            noheaders,
            separator: sep,
            columns,
            format_name: "TSV",
            input,
            head,
            content_type: Some(mime::TEXT_TAB_SEPARATED_VALUES.to_string()),
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
    fn test_examples() {
        use crate::test_examples;

        test_examples(ToTsv {})
    }

    #[test]
    fn test_content_type_metadata() {
        let mut engine_state = Box::new(EngineState::new());
        let delta = {
            // Base functions that are needed for testing
            // Try to keep this working set small to keep tests running as fast as possible
            let mut working_set = StateWorkingSet::new(&engine_state);

            working_set.add_decl(Box::new(ToTsv {}));
            working_set.add_decl(Box::new(Metadata {}));
            working_set.add_decl(Box::new(Get {}));

            working_set.render()
        };

        engine_state
            .merge_delta(delta)
            .expect("Error merging delta");

        let cmd = "{a: 1 b: 2} | to tsv | metadata | get content_type | $in";
        let result = eval_pipeline_without_terminal_expression(
            cmd,
            std::env::temp_dir().as_ref(),
            &mut engine_state,
        );
        assert_eq!(
            Value::test_string("text/tab-separated-values"),
            result.expect("There should be a result")
        );
    }
}
