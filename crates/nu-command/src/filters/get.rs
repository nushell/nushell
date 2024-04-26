use nu_engine::command_prelude::*;

#[derive(Clone)]
pub struct Get;

impl Command for Get {
    fn name(&self) -> &str {
        "get"
    }

    fn usage(&self) -> &str {
        "Extract data using a cell path."
    }

    fn extra_usage(&self) -> &str {
        r#"This is equivalent to using the cell path access syntax: `$env.OS` is the same as `$env | get OS`.

If multiple cell paths are given, this will produce a list of values."#
    }

    fn signature(&self) -> nu_protocol::Signature {
        Signature::build("get")
            .input_output_types(vec![
                (
                    // TODO: This is too permissive; if we could express this
                    // using a type parameter it would be List<T> -> T.
                    Type::List(Box::new(Type::Any)),
                    Type::Any,
                ),
                (Type::table(), Type::Any),
                (Type::record(), Type::Any),
            ])
            .required(
                "cell_path",
                SyntaxShape::CellPath,
                "The cell path to the data.",
            )
            .rest("rest", SyntaxShape::CellPath, "Additional cell paths.")
            .switch(
                "ignore-errors",
                "ignore missing data (make all cell path members optional)",
                Some('i'),
            )
            .switch(
                "sensitive",
                "get path in a case sensitive manner",
                Some('s'),
            )
            .allow_variants_without_examples(true)
            .category(Category::Filters)
    }

    fn run(
        &self,
        engine_state: &EngineState,
        stack: &mut Stack,
        call: &Call,
        input: PipelineData,
    ) -> Result<PipelineData, ShellError> {
        let span = call.head;
        let mut cell_path: CellPath = call.req(engine_state, stack, 0)?;
        let mut rest: Vec<CellPath> = call.rest(engine_state, stack, 1)?;
        let ignore_errors = call.has_flag(engine_state, stack, "ignore-errors")?;
        let sensitive = call.has_flag(engine_state, stack, "sensitive")?;
        let ctrlc = engine_state.ctrlc.clone();
        let metadata = input.metadata();

        if ignore_errors {
            cell_path.make_optional();
            for path in &mut rest {
                path.make_optional();
            }
        }

        if rest.is_empty() {
            input
                .follow_cell_path(&cell_path.members, call.head, !sensitive)
                .map(|x| x.into_pipeline_data())
        } else {
            let mut output = vec![];

            let paths = std::iter::once(cell_path).chain(rest);

            let input = input.into_value(span);

            for path in paths {
                let val = input.clone().follow_cell_path(&path.members, !sensitive);

                output.push(val?);
            }

            Ok(output.into_iter().into_pipeline_data(ctrlc))
        }
        .map(|x| x.set_metadata(metadata))
    }
    fn examples(&self) -> Vec<Example> {
        vec![
            Example {
                description: "Get an item from a list",
                example: "[0 1 2] | get 1",
                result: Some(Value::test_int(1)),
            },
            Example {
                description: "Get a column from a table",
                example: "[{A: A0}] | get A",
                result: Some(Value::list(
                    vec![Value::test_string("A0")],
                    Span::test_data(),
                )),
            },
            Example {
                description: "Get a cell from a table",
                example: "[{A: A0}] | get 0.A",
                result: Some(Value::test_string("A0")),
            },
            Example {
                description:
                    "Extract the name of the 3rd record in a list (same as `ls | $in.name`)",
                example: "ls | get name.2",
                result: None,
            },
            Example {
                description: "Extract the name of the 3rd record in a list",
                example: "ls | get 2.name",
                result: None,
            },
            Example {
                description: "Extract the cpu list from the sys information record",
                example: "sys | get cpu",
                result: None,
            },
            Example {
                description: "Getting Path/PATH in a case insensitive way",
                example: "$env | get paTH",
                result: None,
            },
            Example {
                description: "Getting Path in a case sensitive way, won't work for 'PATH'",
                example: "$env | get --sensitive Path",
                result: None,
            },
        ]
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_examples() {
        use crate::test_examples;

        test_examples(Get)
    }
}
