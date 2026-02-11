use crate::NushellPrompt;
use log::{trace, warn};
use nu_engine::ClosureEvalOnce;
use nu_protocol::{
    Config, PipelineData, Value,
    engine::{EngineState, Stack},
    report_shell_error,
};
use reedline::Prompt;

// Name of environment variable where the prompt could be stored
pub(crate) const PROMPT_COMMAND: &str = "PROMPT_COMMAND";
pub(crate) const PROMPT_COMMAND_RIGHT: &str = "PROMPT_COMMAND_RIGHT";
pub(crate) const PROMPT_INDICATOR: &str = "PROMPT_INDICATOR";
pub(crate) const PROMPT_INDICATOR_VI_INSERT: &str = "PROMPT_INDICATOR_VI_INSERT";
pub(crate) const PROMPT_INDICATOR_VI_NORMAL: &str = "PROMPT_INDICATOR_VI_NORMAL";
pub(crate) const PROMPT_MULTILINE_INDICATOR: &str = "PROMPT_MULTILINE_INDICATOR";
pub(crate) const TRANSIENT_PROMPT_COMMAND: &str = "TRANSIENT_PROMPT_COMMAND";
pub(crate) const TRANSIENT_PROMPT_COMMAND_RIGHT: &str = "TRANSIENT_PROMPT_COMMAND_RIGHT";
pub(crate) const TRANSIENT_PROMPT_INDICATOR: &str = "TRANSIENT_PROMPT_INDICATOR";
pub(crate) const TRANSIENT_PROMPT_INDICATOR_VI_INSERT: &str =
    "TRANSIENT_PROMPT_INDICATOR_VI_INSERT";
pub(crate) const TRANSIENT_PROMPT_INDICATOR_VI_NORMAL: &str =
    "TRANSIENT_PROMPT_INDICATOR_VI_NORMAL";
pub(crate) const TRANSIENT_PROMPT_MULTILINE_INDICATOR: &str =
    "TRANSIENT_PROMPT_MULTILINE_INDICATOR";

// ────────────────────────────────────────────────────────────────────────────────
// OSC 133 / OSC 633 COMMAND EXECUTION MARKERS
// ────────────────────────────────────────────────────────────────────────────────
// These escape sequences are used by the shell to mark command execution boundaries.
// Note: A/B/P markers for prompts are now handled by reedline.

// Command execution markers (C = pre-exec, D = post-exec with exit code)
pub(crate) const PRE_EXECUTION_MARKER: &str = "\x1b]133;C\x1b\\";
pub(crate) const POST_EXECUTION_MARKER_PREFIX: &str = "\x1b]133;D;";
pub(crate) const POST_EXECUTION_MARKER_SUFFIX: &str = "\x1b\\";

// VS Code specific markers (OSC 633)
pub(crate) const VSCODE_PRE_EXECUTION_MARKER: &str = "\x1b]633;C\x1b\\";
pub(crate) const VSCODE_POST_EXECUTION_MARKER_PREFIX: &str = "\x1b]633;D;";
pub(crate) const VSCODE_POST_EXECUTION_MARKER_SUFFIX: &str = "\x1b\\";
pub(crate) const VSCODE_COMMANDLINE_MARKER_PREFIX: &str = "\x1b]633;E;";
pub(crate) const VSCODE_COMMANDLINE_MARKER_SUFFIX: &str = "\x1b\\";
pub(crate) const VSCODE_CWD_PROPERTY_MARKER_PREFIX: &str = "\x1b]633;P;Cwd=";
pub(crate) const VSCODE_CWD_PROPERTY_MARKER_SUFFIX: &str = "\x1b\\";

// Reset terminal application mode sequence
pub(crate) const RESET_APPLICATION_MODE: &str = "\x1b[?1l";

fn get_prompt_string(
    prompt: &str,
    config: &Config,
    engine_state: &EngineState,
    stack: &mut Stack,
) -> Option<String> {
    stack
        .get_env_var(engine_state, prompt)
        .and_then(|v| match v {
            Value::Closure { val, .. } => {
                let result = ClosureEvalOnce::new(engine_state, stack, val.as_ref().clone())
                    .run_with_input(PipelineData::empty());

                trace!(
                    "get_prompt_string (block) {}:{}:{}",
                    file!(),
                    line!(),
                    column!()
                );

                result
                    .map_err(|err| {
                        report_shell_error(None, engine_state, &err);
                    })
                    .ok()
            }
            Value::String { .. } => Some(PipelineData::value(v.clone(), None)),
            _ => None,
        })
        .and_then(|pipeline_data| {
            let output = pipeline_data.collect_string("", config).ok();
            let ansi_output = output.map(|mut x| {
                // Always reset the color at the start of the right prompt
                // to ensure there is no ansi bleed over
                if x.is_empty() && prompt == PROMPT_COMMAND_RIGHT {
                    x.insert_str(0, "\x1b[0m")
                };

                x
            });
            // Let's keep this for debugging purposes with nu --log-level warn
            warn!("{}:{}:{} {:?}", file!(), line!(), column!(), ansi_output);

            ansi_output
        })
}

pub fn update_prompt(
    config: &Config,
    engine_state: &EngineState,
    stack: &mut Stack,
    nu_prompt: &mut NushellPrompt,
) {
    // Get the configured prompts - reedline now handles semantic markers
    let left_prompt_string = get_prompt_string(PROMPT_COMMAND, config, engine_state, stack);

    let right_prompt_string = get_prompt_string(PROMPT_COMMAND_RIGHT, config, engine_state, stack);

    let prompt_indicator_string = get_prompt_string(PROMPT_INDICATOR, config, engine_state, stack);

    let prompt_multiline_string =
        get_prompt_string(PROMPT_MULTILINE_INDICATOR, config, engine_state, stack);

    let prompt_vi_insert_string =
        get_prompt_string(PROMPT_INDICATOR_VI_INSERT, config, engine_state, stack);

    let prompt_vi_normal_string =
        get_prompt_string(PROMPT_INDICATOR_VI_NORMAL, config, engine_state, stack);

    // apply the other indicators
    nu_prompt.update_all_prompt_strings(
        left_prompt_string,
        right_prompt_string,
        prompt_indicator_string,
        prompt_multiline_string,
        (prompt_vi_insert_string, prompt_vi_normal_string),
        config.render_right_prompt_on_last_line,
    );
    trace!("update_prompt {}:{}:{}", file!(), line!(), column!());
}

/// Construct the transient prompt based on the normal nu_prompt
/// Note: Transient prompts do NOT emit semantic markers since they replace
/// the actual prompt after command execution (which already has markers).
pub(crate) fn make_transient_prompt(
    config: &Config,
    engine_state: &EngineState,
    stack: &mut Stack,
    nu_prompt: &NushellPrompt,
) -> Box<dyn Prompt> {
    let mut nu_prompt = nu_prompt.clone();

    if let Some(s) = get_prompt_string(TRANSIENT_PROMPT_COMMAND, config, engine_state, stack) {
        nu_prompt.update_prompt_left(Some(s))
    }

    if let Some(s) = get_prompt_string(TRANSIENT_PROMPT_COMMAND_RIGHT, config, engine_state, stack)
    {
        nu_prompt.update_prompt_right(Some(s), config.render_right_prompt_on_last_line)
    }

    if let Some(s) = get_prompt_string(TRANSIENT_PROMPT_INDICATOR, config, engine_state, stack) {
        nu_prompt.update_prompt_indicator(Some(s))
    }
    if let Some(s) = get_prompt_string(
        TRANSIENT_PROMPT_INDICATOR_VI_INSERT,
        config,
        engine_state,
        stack,
    ) {
        nu_prompt.update_prompt_vi_insert(Some(s))
    }
    if let Some(s) = get_prompt_string(
        TRANSIENT_PROMPT_INDICATOR_VI_NORMAL,
        config,
        engine_state,
        stack,
    ) {
        nu_prompt.update_prompt_vi_normal(Some(s))
    }

    if let Some(s) = get_prompt_string(
        TRANSIENT_PROMPT_MULTILINE_INDICATOR,
        config,
        engine_state,
        stack,
    ) {
        nu_prompt.update_prompt_multiline(Some(s))
    }

    Box::new(nu_prompt)
}

#[cfg(test)]
mod tests {
    use super::*;
    use nu_protocol::Span;

    #[test]
    fn update_prompt_does_not_embed_osc_markers() {
        let mut config = Config::default();
        config.shell_integration.osc133 = true;

        let engine_state = EngineState::new();
        let mut stack = Stack::new();
        stack.add_env_var(
            PROMPT_COMMAND.into(),
            Value::string("test", Span::unknown()),
        );

        let mut nu_prompt = NushellPrompt::new();

        update_prompt(&config, &engine_state, &mut stack, &mut nu_prompt);

        assert_eq!(nu_prompt.render_prompt_left(), "test");
    }
}
