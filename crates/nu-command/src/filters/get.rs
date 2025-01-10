use nu_engine::command_prelude::*;
use nu_protocol::{ast::PathMember, Signals};

#[derive(Clone)]
pub struct Get;

impl Command for Get {
    fn name(&self) -> &str {
        "get"
    }

    fn description(&self) -> &str {
        "Extract data using a cell path."
    }

    fn extra_description(&self) -> &str {
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
                (Type::Nothing, Type::Nothing),
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
                    "Extract the name of the 3rd record in a list (same as `ls | $in.name.2`)",
                example: "ls | get name.2",
                result: None,
            },
            Example {
                description: "Extract the name of the 3rd record in a list",
                example: "ls | get 2.name",
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

    fn is_const(&self) -> bool {
        true
    }

    fn run_const(
        &self,
        working_set: &StateWorkingSet,
        call: &Call,
        input: PipelineData,
    ) -> Result<PipelineData, ShellError> {
        let cell_path: CellPath = call.req_const(working_set, 0)?;
        let rest: Vec<CellPath> = call.rest_const(working_set, 1)?;
        let ignore_errors = call.has_flag_const(working_set, "ignore-errors")?;
        let sensitive = call.has_flag_const(working_set, "sensitive")?;
        let metadata = input.metadata();
        action(
            input,
            cell_path,
            rest,
            ignore_errors,
            sensitive,
            working_set.permanent().signals().clone(),
            call.head,
        )
        .map(|x| x.set_metadata(metadata))
    }

    fn run(
        &self,
        engine_state: &EngineState,
        stack: &mut Stack,
        call: &Call,
        input: PipelineData,
    ) -> Result<PipelineData, ShellError> {
        let cell_path: CellPath = call.req(engine_state, stack, 0)?;
        let rest: Vec<CellPath> = call.rest(engine_state, stack, 1)?;
        let ignore_errors = call.has_flag(engine_state, stack, "ignore-errors")?;
        let sensitive = call.has_flag(engine_state, stack, "sensitive")?;
        let metadata = input.metadata();
        action(
            input,
            cell_path,
            rest,
            ignore_errors,
            sensitive,
            engine_state.signals().clone(),
            call.head,
        )
        .map(|x| x.set_metadata(metadata))
    }
}

fn action(
    input: PipelineData,
    mut cell_path: CellPath,
    mut rest: Vec<CellPath>,
    ignore_errors: bool,
    sensitive: bool,
    signals: Signals,
    span: Span,
) -> Result<PipelineData, ShellError> {
    if ignore_errors {
        cell_path.make_optional();
        for path in &mut rest {
            path.make_optional();
        }
    }

    match input {
        PipelineData::Empty => return Err(ShellError::PipelineEmpty { dst_span: span }),
        // Allow chaining of get -i
        PipelineData::Value(val @ Value::Nothing { .. }, ..) if !ignore_errors => {
            return Err(ShellError::OnlySupportsThisInputType {
                exp_input_type: "table or record".into(),
                wrong_type: "nothing".into(),
                dst_span: span,
                src_span: val.span(),
            })
        }
        _ => (),
    }

    if rest.is_empty() {
        follow_cell_path_into_stream(input, signals, cell_path.members, span, !sensitive)
    } else {
        let mut output = vec![];

        let paths = std::iter::once(cell_path).chain(rest);

        let input = input.into_value(span)?;

        for path in paths {
            let val = input.clone().follow_cell_path(&path.members, !sensitive);

            output.push(val?);
        }

        Ok(output.into_iter().into_pipeline_data(span, signals))
    }
}

// the PipelineData.follow_cell_path function, when given a
// stream, collects it into a vec before doing its job
//
// this is fine, since it returns a Result<Value ShellError>,
// but if we want to follow a PipelineData into a cell path and
// return another PipelineData, then we have to take care to
// make sure it streams
pub fn follow_cell_path_into_stream(
    data: PipelineData,
    signals: Signals,
    cell_path: Vec<PathMember>,
    head: Span,
    insensitive: bool,
) -> Result<PipelineData, ShellError> {
    // when given an integer/indexing, we fallback to
    // the default nushell indexing behaviour
    let has_int_member = cell_path
        .iter()
        .any(|it| matches!(it, PathMember::Int { .. }));
    match data {
        PipelineData::ListStream(stream, ..) if !has_int_member => {
            let result = stream
                .into_iter()
                .map(move |value| {
                    let span = value.span();

                    match value.follow_cell_path(&cell_path, insensitive) {
                        Ok(v) => v,
                        Err(error) => Value::error(error, span),
                    }
                })
                .into_pipeline_data(head, signals);

            Ok(result)
        }

        _ => data
            .follow_cell_path(&cell_path, head, insensitive)
            .map(|x| x.into_pipeline_data()),
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
