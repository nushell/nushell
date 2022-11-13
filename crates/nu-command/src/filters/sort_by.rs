use nu_engine::CallExt;
use nu_protocol::{
    ast::Call,
    engine::{Command, EngineState, Stack},
    Category, Example, IntoInterruptiblePipelineData, PipelineData, ShellError, Signature, Span,
    SyntaxShape, Type, Value,
};

#[derive(Clone)]
pub struct SortBy;

impl Command for SortBy {
    fn name(&self) -> &str {
        "sort-by"
    }

    fn signature(&self) -> nu_protocol::Signature {
        Signature::build("sort-by")
            .input_output_types(vec![(Type::Table(vec![]), Type::Table(vec![]))])
            .rest("columns", SyntaxShape::Any, "the column(s) to sort by")
            .switch("reverse", "Sort in reverse order", Some('r'))
            .switch(
                "insensitive",
                "Sort string-based columns case-insensitively",
                Some('i'),
            )
            .switch(
                "natural",
                "Sort alphanumeric string-based columns naturally",
                Some('n'),
            )
            .category(Category::Filters)
    }

    fn usage(&self) -> &str {
        "Sort by the given columns, in increasing order."
    }

    fn examples(&self) -> Vec<Example> {
        vec![
            Example {
                description: "Sort files by modified date",
                example: "ls | sort-by modified",
                result: None,
            },
            Example {
                description: "Sort files by name (case-insensitive)",
                example: "ls | sort-by name -i",
                result: None,
            },
            Example {
                description: "Sort a table by a column (reversed order)",
                example: "[[fruit count]; [apple 9] [pear 3] [orange 7]] | sort-by fruit -r",
                result: Some(Value::List {
                    vals: vec![
                        Value::test_record(
                            vec!["fruit", "count"],
                            vec![Value::test_string("pear"), Value::test_int(3)],
                        ),
                        Value::test_record(
                            vec!["fruit", "count"],
                            vec![Value::test_string("orange"), Value::test_int(7)],
                        ),
                        Value::test_record(
                            vec!["fruit", "count"],
                            vec![Value::test_string("apple"), Value::test_int(9)],
                        ),
                    ],
                    span: Span::test_data(),
                }),
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
        let columns: Vec<String> = call.rest(engine_state, stack, 0)?;
        let reverse = call.has_flag("reverse");
        let insensitive = call.has_flag("insensitive");
        let natural = call.has_flag("natural");
        let metadata = &input.metadata();
        let mut vec: Vec<_> = input.into_iter().collect();

        if columns.is_empty() {
            return Err(ShellError::MissingParameter("columns".into(), call.head));
        }

        crate::sort(&mut vec, columns, call.head, insensitive, natural)?;

        if reverse {
            vec.reverse()
        }

        let iter = vec.into_iter();
        match metadata {
            Some(m) => {
                Ok(iter.into_pipeline_data_with_metadata(m.clone(), engine_state.ctrlc.clone()))
            }
            None => Ok(iter.into_pipeline_data(engine_state.ctrlc.clone())),
        }
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test_examples() {
        use crate::test_examples;

        test_examples(SortBy {})
    }
}
