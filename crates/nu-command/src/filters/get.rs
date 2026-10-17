use std::borrow::Cow;

use nu_engine::command_prelude::*;
use nu_protocol::{
    DeprecationEntry, DeprecationType, ReportMode, Signals,
    ast::{self, PathMember},
    engine::CallImpl,
};

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
        "This is equivalent to using the cell path access syntax: `$env.OS` is the same as `$env | get OS`.

With no cell path, the input is returned unchanged. With one literal cell path, this returns the value at that path. If multiple cell paths are given, or any argument is a spread (`...$paths`), this produces a list with one value per path, so the result has the same shape however many paths the spread holds, and an empty spread produces an empty list.

Strings in a spread are single column names: `...[a.b]` looks for a column named `a.b`. To spread nested paths, use cell-path values such as `$.a.b`, or convert with `into cell-path`."
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
            .rest("rest", SyntaxShape::CellPath, "The cell paths to the data.")
            .switch(
                "optional",
                "Make all cell path members optional (returns `null` for missing values).",
                Some('o'),
            )
            .switch(
                "ignore-case",
                "Make all cell path members case insensitive.",
                None,
            )
            .switch(
                "ignore-errors",
                "Ignore missing data (make all cell path members optional) (deprecated).",
                Some('i'),
            )
            .switch(
                "sensitive",
                "Get path in a case sensitive manner (deprecated).",
                Some('s'),
            )
            .allow_variants_without_examples(true)
            .category(Category::Filters)
    }

    fn examples(&self) -> Vec<Example<'_>> {
        vec![
            Example {
                description: "Get an item from a list.",
                example: "[0 1 2] | get 1",
                result: Some(Value::test_int(1)),
            },
            Example {
                description: "Return the input unchanged if no cell path is given.",
                example: "[0 1 2] | get",
                result: Some(Value::test_list(vec![
                    Value::test_int(0),
                    Value::test_int(1),
                    Value::test_int(2),
                ])),
            },
            Example {
                description: "Get a column from a table.",
                example: "[{A: A0}] | get A",
                result: Some(Value::list(
                    vec![Value::test_string("A0")],
                    Span::test_data(),
                )),
            },
            Example {
                description: "Get a column from a table where some rows don't have that column, using optional cell-path syntax.",
                example: "[{A: A0, B: B0}, {B: B1}, {A: A2, B: B2}] | get A?",
                result: Some(Value::list(
                    vec![
                        Value::test_string("A0"),
                        Value::test_nothing(),
                        Value::test_string("A2"),
                    ],
                    Span::test_data(),
                )),
            },
            Example {
                description: "Get a column from a table where some rows don't have that column, using the optional flag.",
                example: "[{A: A0, B: B0}, {B: B1}, {A: A2, B: B2}] | get -o A",
                result: Some(Value::list(
                    vec![
                        Value::test_string("A0"),
                        Value::test_nothing(),
                        Value::test_string("A2"),
                    ],
                    Span::test_data(),
                )),
            },
            Example {
                description: "Get a cell from a table.",
                example: "[{A: A0}] | get 0.A",
                result: Some(Value::test_string("A0")),
            },
            Example {
                description: "Get multiple columns from a table using a spread list.",
                example: "[[a b c]; [1 2 3]] | get ...[a c]",
                result: Some(Value::test_list(vec![
                    Value::test_list(vec![Value::test_int(1)]),
                    Value::test_list(vec![Value::test_int(3)]),
                ])),
            },
            Example {
                description: "Get columns from a table using a spread variable.",
                example: "let cols = [a b]; [{a: 1, b: 2, c: 3}] | get ...$cols",
                result: Some(Value::test_list(vec![
                    Value::test_list(vec![Value::test_int(1)]),
                    Value::test_list(vec![Value::test_int(2)]),
                ])),
            },
            Example {
                description: "A spread always produces a list of values, even when it holds a single path.",
                example: "let cols = [a]; [{a: 1, b: 2}] | get ...$cols",
                result: Some(Value::test_list(vec![Value::test_list(vec![
                    Value::test_int(1),
                ])])),
            },
            Example {
                description: "An empty spread produces an empty list.",
                example: "let cols = []; [{a: 1, b: 2}] | get ...$cols",
                result: Some(Value::test_list(vec![])),
            },
            Example {
                description: "Spread nested paths as cell-path values, since spread strings are single column names.",
                example: "let paths = [$.a.b]; {a: {b: 1}} | get ...$paths",
                result: Some(Value::test_list(vec![Value::test_int(1)])),
            },
            Example {
                description: "Extract the name of the 3rd record in a list (same as `ls | $in.name.2`).",
                example: "ls | get name.2",
                result: None,
            },
            Example {
                description: "Extract the name of the 3rd record in a list.",
                example: "ls | get 2.name",
                result: None,
            },
            Example {
                description: "Getting environment variables in a case insensitive way, using case insensitive cell-path syntax.",
                example: "$env | get home! path!",
                result: None,
            },
            Example {
                description: "Getting environment variables in a case insensitive way, using the '--ignore-case' flag.",
                example: "$env | get --ignore-case home path",
                result: None,
            },
            Example {
                description: "Getting Path in a case sensitive way, won't work for 'PATH'.",
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
        stack: &mut Stack,
        call: &Call,
        input: PipelineData,
    ) -> Result<PipelineData, ShellError> {
        let cell_paths = call.rest_const(working_set, stack, 0)?;
        let has_spread = has_spread_argument(call, stack);
        let optional = call.has_flag_const(working_set, stack, "optional")?
            || call.has_flag_const(working_set, stack, "ignore-errors")?;
        let ignore_case = call.has_flag_const(working_set, stack, "ignore-case")?;
        action(
            input,
            cell_paths,
            has_spread,
            optional,
            ignore_case,
            working_set.permanent().signals().clone(),
            call.head,
        )
    }

    fn run(
        &self,
        engine_state: &EngineState,
        stack: &mut Stack,
        call: &Call,
        input: PipelineData,
    ) -> Result<PipelineData, ShellError> {
        let cell_paths = call.rest(engine_state, stack, 0)?;
        let has_spread = has_spread_argument(call, stack);
        let optional = call.has_flag(engine_state, stack, "optional")?
            || call.has_flag(engine_state, stack, "ignore-errors")?;
        let ignore_case = call.has_flag(engine_state, stack, "ignore-case")?;
        action(
            input,
            cell_paths,
            has_spread,
            optional,
            ignore_case,
            engine_state.signals().clone(),
            call.head,
        )
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

/// Returns whether any argument to the rest parameter is a spread (`...$paths`).
///
/// A spread's length is only known at runtime, so `get` always returns a list for it
/// rather than letting a one-element spread collapse to a single value.
fn has_spread_argument(call: &Call, stack: &Stack) -> bool {
    match &call.inner {
        CallImpl::AstRef(call) => ast_has_spread(call),
        CallImpl::AstBox(call) => ast_has_spread(call),
        CallImpl::IrRef(call) => call.rest_iter(stack, 0).any(|(_, spread)| spread),
        CallImpl::IrBox(call) => call.rest_iter(stack, 0).any(|(_, spread)| spread),
    }
}

fn ast_has_spread(call: &ast::Call) -> bool {
    call.arguments
        .iter()
        .any(|arg| matches!(arg, ast::Argument::Spread(_)))
}

/// Follows `cell_paths` into `input`.
///
/// With no path and no spread, the input is returned unchanged, after the same
/// empty-pipeline check the other cases get. One path with no spread returns the value at
/// that path, streaming where possible. Several paths, or any spread, return a list with
/// one value per path, so an empty spread returns an empty list.
fn action(
    mut input: PipelineData,
    mut cell_paths: Vec<CellPath>,
    has_spread: bool,
    optional: bool,
    ignore_case: bool,
    signals: Signals,
    span: Span,
) -> Result<PipelineData, ShellError> {
    for path in &mut cell_paths {
        if optional {
            path.make_optional();
        }
        if ignore_case {
            path.make_insensitive();
        }
    }

    if let PipelineData::Empty = input {
        return Err(ShellError::PipelineEmpty { dst_span: span });
    }

    if cell_paths.is_empty() && !has_spread {
        return Ok(input);
    }

    let mut metadata = input.take_metadata();

    if cell_paths.len() == 1 && !has_spread {
        let cell_path = cell_paths.remove(0);
        let cell_path_is_index = matches!(&cell_path.members[..], &[PathMember::Int { .. }]);
        follow_cell_path_into_stream(input, signals, cell_path.members, span).map(|data| {
            if !cell_path_is_index && let Some(metadata) = &mut metadata {
                metadata.path_columns.clear();
            }
            data.set_metadata(metadata)
        })
    } else {
        let input = input.into_value(span)?;

        let mut output = Vec::with_capacity(cell_paths.len());
        let mut any_cell_path_is_index = false;

        for path in cell_paths {
            any_cell_path_is_index |= matches!(&path.members[..], &[PathMember::Int { .. }]);
            output.push(input.follow_cell_path(&path.members)?.into_owned());
        }

        if !any_cell_path_is_index && let Some(metadata) = &mut metadata {
            metadata.path_columns.clear();
        }

        Ok(output
            .into_iter()
            .into_pipeline_data_with_metadata(span, signals, metadata))
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
    fn test_examples() -> nu_test_support::Result {
        nu_test_support::test().examples(Get)
    }
}
