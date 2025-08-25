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
        "The overlays are listed in the order they were activated. Hidden overlays are listed first, followed by active overlays listed in the order that they were activated. `last` command will always give the top active overlay"
    }

    fn run(
        &self,
        engine_state: &EngineState,
        stack: &mut Stack,
        call: &Call,
        _input: PipelineData,
    ) -> Result<PipelineData, ShellError> {
        // get active overlay iterator
        let active_overlays = stack
            .active_overlays
            .iter()
            .map(|overlay| (overlay.clone(), true));

        // Get all overlay names from engine state
        let output_rows: Vec<Value> = engine_state
            .scope
            .overlays
            .iter()
            .filter_map(|(name, _)| {
                let name = String::from_utf8_lossy(name).to_string();
                if stack
                    .active_overlays
                    .iter()
                    .any(|active_name| active_name == &name)
                {
                    None
                } else {
                    Some((name, false))
                }
            })
            .chain(active_overlays)
            .map(|(name, active)| {
                Value::record(
                    record! {
                        "name" => Value::string(name.to_owned(), call.head),
                        "active" => Value::bool(active, call.head),
                    },
                    call.head,
                )
            })
            .collect();

        Ok(Value::list(output_rows, call.head).into_pipeline_data())
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
