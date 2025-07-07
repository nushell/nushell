use nu_engine::command_prelude::*;

#[derive(Clone)]
pub struct OverlayList;

impl Command for OverlayList {
    fn name(&self) -> &str {
        "overlay list"
    }

    fn description(&self) -> &str {
        "List all active overlays, or hidden ones with --hidden."
    }

    fn signature(&self) -> nu_protocol::Signature {
        Signature::build("overlay list")
            .category(Category::Core)
            .switch("hidden", "Show hidden overlays", None)
            .input_output_types(vec![(Type::Nothing, Type::List(Box::new(Type::String)))])
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
        let show_hidden = call.has_flag(engine_state, stack, "hidden")?;

        let overlays: Vec<Value> = if show_hidden {
            // Show all overlays from engine state (including hidden ones)
            engine_state
                .scope
                .overlays
                .iter()
                .filter_map(|(name, _)| {
                    let overlay_name = String::from_utf8_lossy(name);
                    stack
                        .active_overlays
                        .iter()
                        .all(|name| name.as_str() != overlay_name) // filter already existing overlays
                        .then_some(Value::string(overlay_name, call.head))
                })
                .collect()
        } else {
            // Show only active overlays from stack
            stack
                .active_overlays
                .iter()
                .map(|s| Value::string(s, call.head))
                .collect()
        };

        Ok(Value::list(overlays, call.head).into_pipeline_data())
    }

    fn examples(&self) -> Vec<Example> {
        vec![
            Example {
                description: "Get the last activated overlay",
                example: r#"module spam { export def foo [] { "foo" } }
    overlay use spam
    overlay list | last"#,
                result: Some(Value::test_string("spam")),
            },
            Example {
                description: "List all overlays including hidden ones",
                example: r#"module spam { export def foo [] { "foo" } }
    overlay use spam
    overlay hide spam
    overlay list --hidden | last"#,
                result: Some(Value::test_string("spam")),
            },
        ]
    }
}
