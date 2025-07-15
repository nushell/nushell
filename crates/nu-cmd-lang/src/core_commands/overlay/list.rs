use nu_engine::command_prelude::*;

#[derive(Clone)]
pub struct OverlayList;

impl Command for OverlayList {
    fn name(&self) -> &str {
        "overlay list"
    }

    fn description(&self) -> &str {
        "List all overlays with their active status."
    }

    fn signature(&self) -> nu_protocol::Signature {
        Signature::build("overlay list")
            .category(Category::Core)
            .input_output_types(vec![(
                Type::Nothing,
                Type::Table(
                    vec![
                        ("name".to_string(), Type::String),
                        ("active".to_string(), Type::Bool),
                    ]
                    .into(),
                ),
            )])
    }

    fn extra_description(&self) -> &str {
        "The overlays are listed in the order they were activated."
    }

    fn run(
        &self,
        engine_state: &EngineState,
        stack: &mut Stack,
        call: &Call,
        _input: PipelineData,
    ) -> Result<PipelineData, ShellError> {
        // Get all overlay names from engine state
        let mut all_overlays: Vec<String> = engine_state
            .scope
            .overlays
            .iter()
            .map(|(name, _)| String::from_utf8_lossy(name).to_string())
            .collect();
        all_overlays.sort();

        // Get active overlay names from stack
        let active_overlays = &stack.active_overlays;

        // Create table rows
        let mut rows: Vec<Value> = Vec::new();
        for overlay_name in all_overlays {
            let is_active = active_overlays.contains(&overlay_name);
            let record = Value::record(
                record! {
                    "name" => Value::string(overlay_name, call.head),
                    "active" => Value::bool(is_active, call.head),
                },
                call.head,
            );
            rows.push(record);
        }

        Ok(Value::list(rows, call.head).into_pipeline_data())
    }

    fn examples(&self) -> Vec<Example> {
        vec![
            Example {
                description: "List all overlays with their active status",
                example: r#"module spam { export def foo [] { "foo" } }
    overlay use spam
    overlay list"#,
                result: Some(Value::test_list(vec![Value::test_record(record! {
                    "name" => Value::test_string("spam"),
                    "active" => Value::test_bool(true),
                })])),
            },
            Example {
                description: "Get overlay status after hiding",
                example: r#"module spam { export def foo [] { "foo" } }
    overlay use spam
    overlay hide spam
    overlay list | where name == "spam""#,
                result: Some(Value::test_list(vec![Value::test_record(record! {
                    "name" => Value::test_string("spam"),
                    "active" => Value::test_bool(false),
                })])),
            },
        ]
    }
}
