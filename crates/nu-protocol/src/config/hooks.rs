use crate::{ShellError, Span, Value};
use serde::{Deserialize, Serialize};
/// Definition of a parsed hook from the config object
#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct Hooks {
    pub pre_prompt: Option<Value>,
    pub pre_execution: Option<Value>,
    pub env_change: Option<Value>,
    pub display_output: Option<Value>,
    pub command_not_found: Option<Value>,
}

impl Hooks {
    pub fn new() -> Self {
        Self {
            pre_prompt: None,
            pre_execution: None,
            env_change: None,
            display_output: Some(Value::string(
                "if (term size).columns >= 100 { table -e } else { table }",
                Span::unknown(),
            )),
            command_not_found: None,
        }
    }
}

impl Default for Hooks {
    fn default() -> Self {
        Self::new()
    }
}

/// Parse the hooks to find the blocks to run when the hooks fire
pub(super) fn create_hooks(value: &Value) -> Result<Hooks, ShellError> {
    let span = value.span();
    match value {
        Value::Record { val, .. } => {
            let mut hooks = Hooks::new();

            for (col, val) in val {
                match col.as_str() {
                    "pre_prompt" => hooks.pre_prompt = Some(val.clone()),
                    "pre_execution" => hooks.pre_execution = Some(val.clone()),
                    "env_change" => hooks.env_change = Some(val.clone()),
                    "display_output" => hooks.display_output = Some(val.clone()),
                    "command_not_found" => hooks.command_not_found = Some(val.clone()),
                    x => {
                        return Err(ShellError::UnsupportedConfigValue(
                        "'pre_prompt', 'pre_execution', 'env_change', 'display_output', 'command_not_found'"
                            .to_string(),
                        x.to_string(),
                        span,
                    ));
                    }
                }
            }

            Ok(hooks)
        }
        _ => Err(ShellError::UnsupportedConfigValue(
            "record for 'hooks' config".into(),
            "non-record value".into(),
            span,
        )),
    }
}
