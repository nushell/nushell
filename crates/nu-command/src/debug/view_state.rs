use nu_engine::{command_prelude::*, scope::ScopeData};

#[derive(Clone)]
pub struct ViewState;

impl Command for ViewState {
    fn name(&self) -> &str {
        "view state"
    }

    fn description(&self) -> &str {
        "View the files registered in nushell's EngineState memory."
    }

    fn extra_description(&self) -> &str {
        "These are files parsed and loaded at runtime."
    }

    fn signature(&self) -> nu_protocol::Signature {
        Signature::build("view files")
            // .input_output_types(vec![(
            //     Type::Nothing,
            //     Type::Table(
            //         [
            //             ("filename".into(), Type::String),
            //             ("start".into(), Type::Int),
            //             ("end".into(), Type::Int),
            //             ("size".into(), Type::Int),
            //         ]
            //         .into(),
            //     ),
            // )])
            .category(Category::Debug)
    }

    fn run(
        &self,
        engine_state: &EngineState,
        stack: &mut Stack,
        call: &Call,
        _input: PipelineData,
    ) -> Result<PipelineData, ShellError> {
        let mut rec = engine_state.get_engine_state_as_record();

        // Get detailed variable information using ScopeData
        let mut scope_data = ScopeData::new(engine_state, stack);
        scope_data.populate_vars();
        let vars_with_details = scope_data.collect_vars(call.head);

        // Append the detailed vars list to the record
        rec.insert(
            "vars".to_string(),
            Value::list(vars_with_details, call.head),
        );

        Ok(Value::record(rec, call.head).into_pipeline_data())
    }

    fn examples(&self) -> Vec<Example<'_>> {
        vec![
            Example {
                description: "View the files registered in Nushell's EngineState memory",
                example: r#"view files"#,
                result: None,
            },
            Example {
                description: "View how Nushell was originally invoked",
                example: r#"view files | get 0"#,
                result: None,
            },
        ]
    }
}
