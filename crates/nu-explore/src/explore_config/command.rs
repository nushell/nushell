//! Command definition for the `explore config` command.

use crate::explore_config::conversion::{
    build_nu_type_map, json_to_nu_value, nu_value_to_json, parse_config_documentation,
};
use crate::explore_config::example_data::get_example_json;
use crate::explore_config::tree::print_json_tree;
use crate::explore_config::tui::run_config_tui;
use crate::explore_config::types::NuValueType;
use nu_engine::command_prelude::*;
use nu_protocol::{IntoValue, PipelineData};
use serde_json::Value;
use std::collections::HashMap;

/// A command to explore and edit nushell configuration interactively.
#[derive(Clone)]
pub struct ExploreConfigCommand;

impl Command for ExploreConfigCommand {
    fn name(&self) -> &str {
        "explore config"
    }

    fn description(&self) -> &str {
        "Launch a TUI to view and edit the nushell configuration interactively."
    }

    fn signature(&self) -> nu_protocol::Signature {
        Signature::build("explore config")
            .input_output_types(vec![
                (Type::Nothing, Type::String),
                (Type::String, Type::String),
            ])
            .switch(
                "use-example-data",
                "Show the nushell configuration TUI using example data",
                Some('e'),
            )
            .switch(
                "tree",
                "Do not show the TUI, just show a tree structure of the data",
                Some('t'),
            )
            .named(
                "output",
                SyntaxShape::String,
                "Optional output file to save changes to (default: output.json)",
                Some('o'),
            )
            .category(Category::Viewers)
    }

    fn extra_description(&self) -> &str {
        r#"By default, opens the current nushell configuration ($env.config) in the TUI.
Changes made in config mode are applied to the running session when you quit.

You can also pipe JSON data to explore arbitrary data structures, or use
--use-example-data to see sample configuration data.

TUI Keybindings:
  Tab           Switch between tree and editor panes
  ↑↓            Navigate tree / scroll editor
  ←→            Collapse/Expand tree nodes
  Enter/Space   Toggle tree node expansion
  Enter/e       Start editing (in editor pane)
  Ctrl+Enter    Apply edit
  Esc           Cancel edit
  Ctrl+S        Save/Apply changes
  q             Quit (applies config changes if modified)
  Ctrl+C        Force quit without saving"#
    }

    fn run(
        &self,
        engine_state: &EngineState,
        stack: &mut Stack,
        call: &Call,
        input: PipelineData,
    ) -> Result<PipelineData, ShellError> {
        let input_span = input.span().unwrap_or(call.head);
        let (string_input, _span, _metadata) = input.collect_string_strict(input_span)?;
        let use_example = call.has_flag(engine_state, stack, "use-example-data")?;
        let cli_mode = call.has_flag(engine_state, stack, "tree")?;
        let output_file: Option<String> = call.get_flag(engine_state, stack, "output")?;

        // Determine the data source and mode
        // nu_type_map is used in config mode to track original nushell types
        // doc_map is used in config mode to show documentation for config options
        let (json_data, config_mode, nu_type_map, doc_map): (
            Value,
            bool,
            Option<HashMap<String, NuValueType>>,
            Option<HashMap<String, String>>,
        ) = if use_example {
            // Use example data
            (get_example_json(), false, None, None)
        } else if !string_input.trim().is_empty() {
            // Use piped input data
            let data =
                serde_json::from_str(&string_input).map_err(|e| ShellError::GenericError {
                    error: "Could not parse JSON from input".into(),
                    msg: format!("JSON parse error: {e}"),
                    span: Some(call.head),
                    help: Some("Make sure the input is valid JSON".into()),
                    inner: vec![],
                })?;
            (data, false, None, None)
        } else {
            // Default: use nushell configuration
            // First convert Config to nu_protocol::Value, then to serde_json::Value
            // This properly handles closures by converting them to their string representation
            let config = stack.get_config(engine_state);
            let nu_value = config.as_ref().clone().into_value(call.head);
            let json_data = nu_value_to_json(engine_state, &nu_value, call.head)?;
            // Build nu_type_map to track original nushell types
            let mut nu_type_map = HashMap::new();
            build_nu_type_map(&nu_value, Vec::new(), &mut nu_type_map);
            // Parse documentation from doc_config.nu
            let doc_map = parse_config_documentation();
            (json_data, true, Some(nu_type_map), Some(doc_map))
        };

        if cli_mode {
            // Original CLI behavior
            print_json_tree(&json_data, "", true, None);
        } else {
            // TUI mode
            let result = run_config_tui(json_data, output_file, config_mode, nu_type_map, doc_map)?;

            // If in config mode and data was modified, apply changes to the config
            if config_mode {
                if let Some(modified_json) = result {
                    // Convert JSON back to nu_protocol::Value
                    let nu_value = json_to_nu_value(&modified_json, call.head).map_err(|e| {
                        ShellError::GenericError {
                            error: "Could not convert JSON to nu Value".into(),
                            msg: format!("conversion error: {e}"),
                            span: Some(call.head),
                            help: None,
                            inner: vec![],
                        }
                    })?;

                    // Update $env.config with the new value
                    stack.add_env_var("config".into(), nu_value);

                    // Apply the config update
                    stack.update_config(engine_state)?;
                }
            }
        }

        Ok(PipelineData::empty())
    }

    fn examples(&self) -> Vec<Example<'_>> {
        vec![
            Example {
                description: "Open the nushell configuration in an interactive TUI editor",
                example: r#"explore config"#,
                result: None,
            },
            Example {
                description: "Explore JSON data interactively",
                example: r#"open data.json | explore config"#,
                result: None,
            },
            Example {
                description: "Explore with example data to see TUI features",
                example: r#"explore config --use-example-data"#,
                result: None,
            },
        ]
    }
}
