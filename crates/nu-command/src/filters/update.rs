use nu_engine::{eval_block, CallExt};
use nu_protocol::ast::{Call, CellPath, PathMember};
use nu_protocol::engine::{Closure, Command, EngineState, Stack};
use nu_protocol::{
    Category, Example, FromValue, IntoInterruptiblePipelineData, IntoPipelineData, PipelineData,
    ShellError, Signature, Span, SyntaxShape, Type, Value,
};

#[derive(Clone)]
pub struct Update;

impl Command for Update {
    fn name(&self) -> &str {
        "update"
    }

    fn signature(&self) -> Signature {
        Signature::build("update")
            .input_output_types(vec![(Type::Table(vec![]), Type::Table(vec![]))])
            .required(
                "field",
                SyntaxShape::CellPath,
                "the name of the column to update",
            )
            .required(
                "replacement value",
                SyntaxShape::Any,
                "the new value to give the cell(s), or a block to create the value",
            )
            .category(Category::Filters)
    }

    fn usage(&self) -> &str {
        "Update an existing column to have a new value."
    }

    fn run(
        &self,
        engine_state: &EngineState,
        stack: &mut Stack,
        call: &Call,
        input: PipelineData,
    ) -> Result<nu_protocol::PipelineData, nu_protocol::ShellError> {
        update(engine_state, stack, call, input)
    }

    fn examples(&self) -> Vec<Example> {
        vec![
            Example {
                description: "Update a column value",
                example: "{'name': 'nu', 'stars': 5} | update name 'Nushell'",
                result: Some(Value::Record {
                    cols: vec!["name".into(), "stars".into()],
                    vals: vec![Value::test_string("Nushell"), Value::test_int(5)],
                    span: Span::test_data(),
                }),
            },
            Example {
                description: "Use in block form for more involved updating logic",
                example: "[[count fruit]; [1 'apple']] | update count {|row index| ($row.fruit | str length) + $index }",
                result: Some(Value::List {
                    vals: vec![Value::Record {
                        cols: vec!["count".into(), "fruit".into()],
                        vals: vec![Value::test_int(5), Value::test_string("apple")],
                        span: Span::test_data(),
                    }],
                    span: Span::test_data(),
                }),
            },
            Example {
                description: "Alter each value in the 'authors' column to use a single string instead of a list",
                example: "[[project, authors]; ['nu', ['Andrés', 'JT', 'Yehuda']]] | update authors {|row| $row.authors | str join ','}",
                result: Some(Value::List { vals: vec![Value::Record { cols: vec!["project".into(), "authors".into()], vals: vec![Value::test_string("nu"), Value::test_string("Andrés,JT,Yehuda")], span: Span::test_data()}], span: Span::test_data()}),
            },
        ]
    }
}

fn update(
    engine_state: &EngineState,
    stack: &mut Stack,
    call: &Call,
    input: PipelineData,
) -> Result<PipelineData, ShellError> {
    let span = call.head;

    let cell_path: CellPath = call.req(engine_state, stack, 0)?;
    let replacement: Value = call.req(engine_state, stack, 1)?;

    let redirect_stdout = call.redirect_stdout;
    let redirect_stderr = call.redirect_stderr;

    let engine_state = engine_state.clone();
    let ctrlc = engine_state.ctrlc.clone();

    // Replace is a block, so set it up and run it instead of using it as the replacement
    if replacement.as_block().is_ok() {
        let capture_block: Closure = FromValue::from_value(&replacement)?;
        let block = engine_state.get_block(capture_block.block_id).clone();

        let mut stack = stack.captures_to_stack(&capture_block.captures);
        let orig_env_vars = stack.env_vars.clone();
        let orig_env_hidden = stack.env_hidden.clone();

        // enumerate() can't be used here because it converts records into tables
        // when combined with into_pipeline_data(). Hence, the index is tracked manually like so.
        let mut idx: i64 = 0;
        input.map(
            move |mut input| {
                // with_env() is used here to ensure that each iteration uses
                // a different set of environment variables.
                // Hence, a 'cd' in the first loop won't affect the next loop.
                stack.with_env(&orig_env_vars, &orig_env_hidden);

                if let Some(var) = block.signature.get_positional(0) {
                    if let Some(var_id) = &var.var_id {
                        stack.add_var(*var_id, input.clone())
                    }
                }
                // Optional index argument
                if let Some(var) = block.signature.get_positional(1) {
                    if let Some(var_id) = &var.var_id {
                        stack.add_var(*var_id, Value::Int { val: idx, span });
                    }
                    idx += 1;
                }

                let output = eval_block(
                    &engine_state,
                    &mut stack,
                    &block,
                    input.clone().into_pipeline_data(),
                    redirect_stdout,
                    redirect_stderr,
                );

                match output {
                    Ok(pd) => {
                        if let Err(e) =
                            input.update_data_at_cell_path(&cell_path.members, pd.into_value(span))
                        {
                            return Value::Error { error: e };
                        }

                        input
                    }
                    Err(e) => Value::Error { error: e },
                }
            },
            ctrlc,
        )
    } else {
        if let Some(PathMember::Int { val, span }) = cell_path.members.get(0) {
            let mut input = input.into_iter();
            let mut pre_elems = vec![];

            for idx in 0..*val {
                if let Some(v) = input.next() {
                    pre_elems.push(v);
                } else if idx == 0 {
                    return Err(ShellError::AccessEmptyContent(*span));
                } else {
                    return Err(ShellError::AccessBeyondEnd(idx - 1, *span));
                }
            }

            // Skip over the replaced value
            let _ = input.next();

            return Ok(pre_elems
                .into_iter()
                .chain(vec![replacement])
                .chain(input)
                .into_pipeline_data(ctrlc));
        }
        input.map(
            move |mut input| {
                let replacement = replacement.clone();

                if let Err(e) = input.update_data_at_cell_path(&cell_path.members, replacement) {
                    return Value::Error { error: e };
                }

                input
            },
            ctrlc,
        )
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test_examples() {
        use crate::test_examples;

        test_examples(Update {})
    }
}
