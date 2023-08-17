use nu_engine::CallExt;
use nu_protocol::{
    ast::Call,
    engine::{Command, EngineState, Stack},
    Category, Example, IntoInterruptiblePipelineData, PipelineData, ShellError, Signature, Span,
    SpannedValue, SyntaxShape, Type,
};

#[derive(Clone)]
pub struct SortBy;

impl Command for SortBy {
    fn name(&self) -> &str {
        "sort-by"
    }

    fn signature(&self) -> nu_protocol::Signature {
        Signature::build("sort-by")
            .input_output_types(vec![
                (Type::Table(vec![]), Type::Table(vec![])),
                (
                    Type::List(Box::new(Type::Any)),
                    Type::List(Box::new(Type::Any)),
                ),
            ])
            .rest("columns", SyntaxShape::Any, "the column(s) to sort by")
            .switch("reverse", "Sort in reverse order", Some('r'))
            .switch(
                "ignore-case",
                "Sort string-based columns case-insensitively",
                Some('i'),
            )
            .switch(
                "natural",
                "Sort alphanumeric string-based columns naturally (1, 9, 10, 99, 100, ...)",
                Some('n'),
            )
            .allow_variants_without_examples(true)
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
                result: Some(SpannedValue::List {
                    vals: vec![
                        SpannedValue::test_record(
                            vec!["fruit", "count"],
                            vec![SpannedValue::test_string("pear"), SpannedValue::test_int(3)],
                        ),
                        SpannedValue::test_record(
                            vec!["fruit", "count"],
                            vec![
                                SpannedValue::test_string("orange"),
                                SpannedValue::test_int(7),
                            ],
                        ),
                        SpannedValue::test_record(
                            vec!["fruit", "count"],
                            vec![
                                SpannedValue::test_string("apple"),
                                SpannedValue::test_int(9),
                            ],
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
        let insensitive = call.has_flag("ignore-case");
        let natural = call.has_flag("natural");
        let metadata = &input.metadata();
        let mut vec: Vec<_> = input.into_iter_strict(call.head)?.collect();

        if columns.is_empty() {
            return Err(ShellError::MissingParameter {
                param_name: "columns".into(),
                span: call.head,
            });
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
