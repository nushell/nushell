use std::borrow::Cow;

use nu_engine::command_prelude::*;
use nu_protocol::{DeprecationEntry, DeprecationType, ReportMode, Signals, ast::PathMember};

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
                "optional",
                "make all cell path members optional (returns `null` for missing values)",
                Some('o'),
            )
            .switch(
                "ignore-errors",
                "ignore missing data (make all cell path members optional) (deprecated)",
                Some('i'),
            )
            .switch(
                "sensitive",
                "get path in a case sensitive manner (deprecated)",
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
                description: "Extract the name of the 3rd record in a list (same as `ls | $in.name.2`)",
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
                example: "$env | get paTH!",
                result: None,
            },
            Example {
                description: "Getting Path in a case sensitive way, won't work for 'PATH'",
                example: "$env | get Path",
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
        let optional = call.has_flag_const(working_set, "optional")?
            || call.has_flag_const(working_set, "ignore-errors")?;
        let metadata = input.metadata();
        action(
            input,
            cell_path,
            rest,
            optional,
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
        let optional = call.has_flag(engine_state, stack, "optional")?
            || call.has_flag(engine_state, stack, "ignore-errors")?;
        let metadata = input.metadata();
        action(
            input,
            cell_path,
            rest,
            optional,
            engine_state.signals().clone(),
            call.head,
        )
        .map(|x| x.set_metadata(metadata))
    }

    fn deprecation_info(&self) -> Vec<DeprecationEntry> {
        vec![
            DeprecationEntry {
                ty: DeprecationType::Flag("sensitive".into()),
                report_mode: ReportMode::FirstUse,
                since: Some("0.105.0".into()),
                expected_removal: None,
                help: Some("Cell-paths are now case-sensitive by default.\nTo access fields case-insensitively, add `!` after the relevant path member.".into())
            },
            DeprecationEntry {
                ty: DeprecationType::Flag("ignore-errors".into()),
                report_mode: ReportMode::FirstUse,
                since: Some("0.106.0".into()),
                expected_removal: None,
                help: Some("This flag has been renamed to `--optional (-o)` to better reflect its behavior.".into())
            }
        ]
    }
}

fn action(
    input: PipelineData,
    mut cell_path: CellPath,
    mut rest: Vec<CellPath>,
    optional: bool,
    signals: Signals,
    span: Span,
) -> Result<PipelineData, ShellError> {
    if optional {
        cell_path.make_optional();
        for path in &mut rest {
            path.make_optional();
        }
    }

    if let PipelineData::Empty = input {
        return Err(ShellError::PipelineEmpty { dst_span: span });
    }

    if rest.is_empty() {
        follow_cell_path_into_stream(input, signals, cell_path.members, span)
    } else {
        let mut output = vec![];

        let paths = std::iter::once(cell_path).chain(rest);

        let input = input.into_value(span)?;

        for path in paths {
            output.push(input.follow_cell_path(&path.members)?.into_owned());
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

                    value
                        .follow_cell_path(&cell_path)
                        .map(Cow::into_owned)
                        .unwrap_or_else(|error| Value::error(error, span))
                })
                .into_pipeline_data(head, signals);

            Ok(result)
        }

        _ => data
            .follow_cell_path(&cell_path, head)
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
