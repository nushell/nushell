use nu_engine::{eval_block, CallExt};
use nu_protocol::ast::{Call, CellPath};
use nu_protocol::engine::{Command, EngineState, Stack};
use nu_protocol::{
    Category, Example, IntoPipelineData, PipelineData, ShellError, Signature, Span, SyntaxShape,
    Value,
};

#[derive(Clone)]
pub struct Update;

impl Command for Update {
    fn name(&self) -> &str {
        "update"
    }

    fn signature(&self) -> Signature {
        Signature::build("update")
            .required(
                "field",
                SyntaxShape::CellPath,
                "the name of the column to update",
            )
            .required(
                "replacement value",
                SyntaxShape::Any,
                "the new value to give the cell(s)",
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
        vec![Example {
            description: "Update a column value",
            example: "echo {'name': 'nu', 'stars': 5} | update name 'Nushell'",
            result: Some(Value::Record { cols: vec!["name".into(), "stars".into()], vals: vec![Value::test_string("Nushell"), Value::test_int(5)], span: Span::test_data()}),
        }, Example {
            description: "Use in block form for more involved updating logic",
            example: "echo [[project, authors]; ['nu', ['Andrés', 'JT', 'Yehuda']]] | update authors { get authors | str collect ',' }",
            result: Some(Value::List { vals: vec![Value::Record { cols: vec!["project".into(), "authors".into()], vals: vec![Value::test_string("nu"), Value::test_string("Andrés,JT,Yehuda")], span: Span::test_data()}], span: Span::test_data()}),
        }]
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
    let engine_state = engine_state.clone();
    let ctrlc = engine_state.ctrlc.clone();

    // Replace is a block, so set it up and run it instead of using it as the replacement
    if let Ok(block_id) = replacement.as_block() {
        let block = engine_state.get_block(block_id).clone();

        let mut stack = stack.collect_captures(&block.captures);
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
                );

                match output {
                    Ok(pd) => {
                        if let Err(e) =
                            input.replace_data_at_cell_path(&cell_path.members, pd.into_value(span))
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

                if let Err(e) = input.replace_data_at_cell_path(&cell_path.members, replacement) {
                    return Value::Error { error: e };
                }

                input
            },
            ctrlc,
        )
    }
}
