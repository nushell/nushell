//! Command definition for the `explore config` command.

use crate::explore_config::conversion::{
    build_nu_type_map, build_original_value_map, json_to_nu_value_with_types, nu_value_to_json,
    parse_config_documentation,
};
use crate::explore_config::example_data::get_example_json;
use crate::explore_config::tree::print_json_tree;
use crate::explore_config::tui::run_config_tui;
use crate::explore_config::types::NuValueType;
use nu_engine::command_prelude::*;
use nu_protocol::{PipelineData, report_shell_warning};
use serde_json::Value;
use std::collections::HashMap;
use std::sync::Arc;

/// Type alias for the tuple returned when determining data source and mode
type ConfigDataResult = (
    Value,
    bool,
    Option<HashMap<String, NuValueType>>,
    Option<HashMap<String, nu_protocol::Value>>,
    Option<HashMap<String, String>>,
);

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
                "Show the nushell configuration TUI using example data.",
                Some('e'),
            )
            .switch(
                "tree",
                "Do not show the TUI, just show a tree structure of the data.",
                Some('t'),
            )
            .named(
                "output",
                SyntaxShape::String,
                "Optional output file to save changes to (default: output.json).",
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
  Enter/Space   On leaf nodes, open editor pane and start editing
  Enter/e       Start editing (in editor pane)
  Ctrl+S        Apply edit
  Alt+Enter     Apply edit (alternative)
  Esc           Cancel edit
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
        // original_values is used to preserve values that can't be roundtripped (closures, dates, etc.)
        // doc_map is used in config mode to show documentation for config options
        let (json_data, config_mode, nu_type_map, original_values, doc_map): ConfigDataResult =
            if use_example {
                // Use example data
                (get_example_json(), false, None, None, None)
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
                (data, false, None, None, None)
            } else {
                // Default: use nushell configuration
                // Get the raw $env.config Value directly to preserve key ordering
                // (using Config::into_value would lose order because HashMap iteration is unordered)
                let nu_value = stack
                    .get_env_var(engine_state, "config")
                    .cloned()
                    .unwrap_or_else(|| {
                        // Fallback to Config struct if $env.config is not set
                        let config = stack.get_config(engine_state);
                        config.as_ref().clone().into_value(call.head)
                    });
                let json_data = nu_value_to_json(engine_state, &nu_value, call.head)?;

                // Build nu_type_map to track original nushell types
                let mut nu_type_map = HashMap::new();
                build_nu_type_map(&nu_value, Vec::new(), &mut nu_type_map);

                // Build original_values map for types that can't be roundtripped (closures, dates, etc.)
                let mut original_values = HashMap::new();
                build_original_value_map(&nu_value, Vec::new(), &mut original_values);

                // Parse documentation from doc_config.nu
                let doc_map = parse_config_documentation();
                (
                    json_data,
                    true,
                    Some(nu_type_map),
                    Some(original_values),
                    Some(doc_map),
                )
            };

        if cli_mode {
            // Original CLI behavior
            print_json_tree(&json_data, "", true, None);
        } else {
            // TUI mode - clone the type map and original values so we can use them after the TUI returns
            let type_map_for_conversion = nu_type_map.clone();
            let original_values_for_conversion = original_values.clone();

            let result = run_config_tui(
                json_data,
                output_file,
                config_mode,
                nu_type_map,
                doc_map,
                Arc::new(engine_state.clone()),
                Arc::new(stack.clone()),
            )?;

            // If in config mode and data was modified, apply changes to the config
            if config_mode && let Some(modified_json) = result {
                // Convert JSON back to nu_protocol::Value, using type map and original values
                // to preserve types like Duration, Filesize, and Closure
                let nu_value = json_to_nu_value_with_types(
                    &modified_json,
                    call.head,
                    &type_map_for_conversion,
                    &original_values_for_conversion,
                    Vec::new(),
                )
                .map_err(|e| ShellError::GenericError {
                    error: "Could not convert JSON to nu Value".into(),
                    msg: format!("conversion error: {e}"),
                    span: Some(call.head),
                    help: None,
                    inner: vec![],
                })?;

                // Update $env.config with the new value
                stack.add_env_var("config".into(), nu_value.clone());

                // Update the internal Config struct directly, without calling update_config()
                // which would overwrite $env.config with Config::into_value() and lose key ordering
                // (because Config uses HashMap for color_config/explore/plugins fields)
                let old_config = stack.get_config(engine_state);
                let mut new_config = (*old_config).clone();
                let result = new_config.update_from_value(&old_config, &nu_value);
                // Store the updated Config struct directly on the stack
                stack.config = Some(Arc::new(new_config));
                if let Some(warning) = result? {
                    report_shell_warning(Some(stack), engine_state, &warning);
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
                example: r#"open --raw data.json | explore config"#,
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
