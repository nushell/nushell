use nu_engine::{eval_block, CallExt};
use nu_protocol::ast::{Call, CellPath, PathMember};
use nu_protocol::engine::{Closure, Command, EngineState, Stack};
use nu_protocol::{
    Category, Example, FromValue, IntoInterruptiblePipelineData, IntoPipelineData, PipelineData,
    ShellError, Signature, Span, SyntaxShape, Type, Value,
};

#[derive(Clone)]
pub struct Insert;

impl Command for Insert {
    fn name(&self) -> &str {
        "insert"
    }

    fn signature(&self) -> Signature {
        Signature::build("insert")
            .input_output_types(vec![
                (Type::Record(vec![]), Type::Record(vec![])),
                // TODO: It accepts table input also (in which case it repeats
                // the value across all table rows) but currently there is no
                // example of the table variant so it cannot be in the
                // signature.
                // (Type::Table(vec![]), Type::Table(vec![])),
            ])
            .required(
                "field",
                SyntaxShape::CellPath,
                "the name of the column to insert",
            )
            .required(
                "new value",
                SyntaxShape::Any,
                "the new value to give the cell(s)",
            )
            .category(Category::Filters)
    }

    fn usage(&self) -> &str {
        "Insert a new column."
    }

    fn search_terms(&self) -> Vec<&str> {
        vec!["add"]
    }

    fn run(
        &self,
        engine_state: &EngineState,
        stack: &mut Stack,
        call: &Call,
        input: PipelineData,
    ) -> Result<nu_protocol::PipelineData, nu_protocol::ShellError> {
        insert(engine_state, stack, call, input)
    }

    fn examples(&self) -> Vec<Example> {
        vec![Example {
            description: "Insert a new entry into a record",
            example: "{'name': 'nu', 'stars': 5} | insert alias 'Nushell'",
            result: Some(Value::Record {
                cols: vec!["name".into(), "stars".into(), "alias".into()],
                vals: vec![
                    Value::test_string("nu"),
                    Value::test_int(5),
                    Value::test_string("Nushell"),
                ],
                span: Span::test_data(),
            }),
        }]
    }
}

fn insert(
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
                            input.insert_data_at_cell_path(&cell_path.members, pd.into_value(span))
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
        if let Some(PathMember::Int { val, .. }) = cell_path.members.get(0) {
            let mut input = input.into_iter();
            let mut pre_elems = vec![];

            for _ in 0..*val {
                if let Some(v) = input.next() {
                    pre_elems.push(v);
                } else {
                    pre_elems.push(Value::Nothing { span })
                }
            }

            return Ok(pre_elems
                .into_iter()
                .chain(vec![replacement])
                .chain(input)
                .into_pipeline_data(ctrlc));
        }
        input.map(
            move |mut input| {
                let replacement = replacement.clone();

                if let Err(e) = input.insert_data_at_cell_path(&cell_path.members, replacement) {
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

        test_examples(Insert {})
    }
}
