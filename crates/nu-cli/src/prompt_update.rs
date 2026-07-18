use crate::NushellPrompt;
use log::{info, trace};
use nu_engine::ClosureEvalOnce;
use nu_protocol::{
    Config, PipelineData, Value,
    engine::{EngineState, PromptContents, PromptState, Stack},
    report_shell_error,
};
use reedline::Prompt;
use std::sync::Arc;

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
    let mut output = match stack.get_env_var(engine_state, prompt)? {
        Value::String { val, .. } => val.clone(),
        Value::Closure { val, .. } => {
            let result = ClosureEvalOnce::new(engine_state, stack, val.as_ref().clone())
                .run_with_input(PipelineData::empty());

            trace!(
                "get_prompt_string (block) {}:{}:{}",
                file!(),
                line!(),
                column!()
            );

            let result_string = result
                .map_err(|err| report_shell_error(None, engine_state, &err))
                .ok()
                .and_then(|pd| pd.collect_string("", config).ok());

            result_string?
        }
        _ => return None,
    };

    // Always reset the color at the start of the right prompt
    // to ensure there is no ansi bleed over
    if output.is_empty() && prompt == PROMPT_COMMAND_RIGHT {
        output.insert_str(0, "\x1b[0m")
    };

    // Let's keep this for debugging purposes with nu --log-level warn
    info!("{}:{}:{} {:?}", file!(), line!(), column!(), output);

    Some(output)
}

/// Re-evaluate `$env.PROMPT_COMMAND` and friends and install the result as the
/// prompt's per-cycle baseline. This overwrites anything a background job pushed
/// during the previous cycle, resetting the prompt for the next line.
pub fn update_prompt(config: &Config, engine_state: &EngineState, stack: &mut Stack) {
    let new_contents = build_prompt_contents(config, engine_state, stack);

    // reedline handles semantic markers itself.
    engine_state.prompt_state.set_contents(new_contents);

    trace!("update_prompt {}:{}:{}", file!(), line!(), column!());
}

fn build_prompt_contents(
    config: &Config,
    engine_state: &EngineState,
    stack: &mut Stack,
) -> PromptContents {
    let mut fetch_prompt =
        |prompt_type| get_prompt_string(prompt_type, config, engine_state, stack);

    PromptContents {
        left: fetch_prompt(PROMPT_COMMAND),
        right: fetch_prompt(PROMPT_COMMAND_RIGHT),
        indicator: fetch_prompt(PROMPT_INDICATOR),
        vi_insert: fetch_prompt(PROMPT_INDICATOR_VI_INSERT),
        vi_normal: fetch_prompt(PROMPT_INDICATOR_VI_NORMAL),
        multiline: fetch_prompt(PROMPT_MULTILINE_INDICATOR),
        render_right_on_last_line: config.render_right_prompt_on_last_line,
    }
}

/// Construct the transient prompt based on the normal nu_prompt
/// Note: Transient prompts do NOT emit semantic markers since they replace
/// the actual prompt after command execution (which already has markers).
pub(crate) fn make_transient_prompt(
    config: &Config,
    engine_state: &EngineState,
    stack: &mut Stack,
) -> Box<dyn Prompt> {
    // Start from the current baseline.
    let mut prompt_contents = engine_state.prompt_state.contents();

    // Map mutable references of the prompt contents to their corresponding config variables.
    let transient_overrides = [
        (&mut prompt_contents.left, TRANSIENT_PROMPT_COMMAND),
        (&mut prompt_contents.right, TRANSIENT_PROMPT_COMMAND_RIGHT),
        (&mut prompt_contents.indicator, TRANSIENT_PROMPT_INDICATOR),
        (
            &mut prompt_contents.vi_insert,
            TRANSIENT_PROMPT_INDICATOR_VI_INSERT,
        ),
        (
            &mut prompt_contents.vi_normal,
            TRANSIENT_PROMPT_INDICATOR_VI_NORMAL,
        ),
        (
            &mut prompt_contents.multiline,
            TRANSIENT_PROMPT_MULTILINE_INDICATOR,
        ),
    ]
    .into_iter();

    transient_overrides.for_each(|(field, env_var)| {
        let val = get_prompt_string(env_var, config, engine_state, stack);

        // Apply overide
        if val.is_some() {
            *field = val;
        }
    });

    // Package
    let detached_state = PromptState::new();
    detached_state.set_contents(prompt_contents);

    Box::new(NushellPrompt { state: Arc::new(detached_state) })
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
            Value::string("test", Span::test_data()),
        );

        update_prompt(&config, &engine_state, &mut stack);

        let nu_prompt = NushellPrompt { state: engine_state.prompt_state.clone() };
        assert_eq!(nu_prompt.render_prompt_left(), "test");
    }
}
