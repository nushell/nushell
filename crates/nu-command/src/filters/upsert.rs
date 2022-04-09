use nu_engine::{eval_block, CallExt};
use nu_protocol::ast::{Call, CellPath};
use nu_protocol::engine::{CaptureBlock, Command, EngineState, Stack};
use nu_protocol::{
    Category, Example, FromValue, IntoPipelineData, PipelineData, ShellError, Signature, Span,
    SyntaxShape, Value,
};

#[derive(Clone)]
pub struct Upsert;

impl Command for Upsert {
    fn name(&self) -> &str {
        "upsert"
    }

    fn signature(&self) -> Signature {
        Signature::build("upsert")
            .required(
                "field",
                SyntaxShape::CellPath,
                "the name of the column to update or insert",
            )
            .required(
                "replacement value",
                SyntaxShape::Any,
                "the new value to give the cell(s)",
            )
            .category(Category::Filters)
    }

    fn usage(&self) -> &str {
        "Update an existing column to have a new value, or insert a new column."
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
        upsert(engine_state, stack, call, input)
    }

    fn examples(&self) -> Vec<Example> {
        vec![Example {
            description: "Update a column value",
            example: "echo {'name': 'nu', 'stars': 5} | upsert name 'Nushell'",
            result: Some(Value::Record { cols: vec!["name".into(), "stars".into()], vals: vec![Value::test_string("Nushell"), Value::test_int(5)], span: Span::test_data()}),
        }, Example {
            description: "Insert a new column",
            example: "echo {'name': 'nu', 'stars': 5} | upsert language 'Rust'",
            result: Some(Value::Record { cols: vec!["name".into(), "stars".into(), "language".into()], vals: vec![Value::test_string("nu"), Value::test_int(5), Value::test_string("Rust")], span: Span::test_data()}),
        }, Example {
            description: "Use in block form for more involved updating logic",
            example: "echo [[count fruit]; [1 'apple']] | upsert count {|f| $f.count + 1}",
            result: Some(Value::List { vals: vec![Value::Record { cols: vec!["count".into(), "fruit".into()], vals: vec![Value::test_int(2), Value::test_string("apple")], span: Span::test_data()}], span: Span::test_data()}),
        }, Example {
            description: "Use in block form for more involved updating logic",
            example: "echo [[project, authors]; ['nu', ['Andrés', 'JT', 'Yehuda']]] | upsert authors {|a| $a.authors | str collect ','}",
            result: Some(Value::List { vals: vec![Value::Record { cols: vec!["project".into(), "authors".into()], vals: vec![Value::test_string("nu"), Value::test_string("Andrés,JT,Yehuda")], span: Span::test_data()}], span: Span::test_data()}),
        }]
    }
}

fn upsert(
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
        let capture_block: CaptureBlock = FromValue::from_value(&replacement)?;
        let block = engine_state.get_block(capture_block.block_id).clone();

        let mut stack = stack.captures_to_stack(&capture_block.captures);
        let orig_env_vars = stack.env_vars.clone();
        let orig_env_hidden = stack.env_hidden.clone();

        input.map(
            move |mut input| {
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
                            input.upsert_data_at_cell_path(&cell_path.members, pd.into_value(span))
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
        input.map(
            move |mut input| {
                let replacement = replacement.clone();

                if let Err(e) = input.upsert_data_at_cell_path(&cell_path.members, replacement) {
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

        test_examples(Upsert {})
    }
}
