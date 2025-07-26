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

// Store all these Ansi Escape Markers here so they can be reused easily
// According to Daniel Imms @Tyriar, we need to do these this way:
// <133 A><prompt><133 B><command><133 C><command output>
pub(crate) const PRE_PROMPT_MARKER: &str = "\x1b]133;A\x1b\\";
pub(crate) const POST_PROMPT_MARKER: &str = "\x1b]133;B\x1b\\";
pub(crate) const PRE_EXECUTION_MARKER: &str = "\x1b]133;C\x1b\\";
pub(crate) const POST_EXECUTION_MARKER_PREFIX: &str = "\x1b]133;D;";
pub(crate) const POST_EXECUTION_MARKER_SUFFIX: &str = "\x1b\\";

// OSC633 is the same as OSC133 but specifically for VSCode
pub(crate) const VSCODE_PRE_PROMPT_MARKER: &str = "\x1b]633;A\x1b\\";
pub(crate) const VSCODE_POST_PROMPT_MARKER: &str = "\x1b]633;B\x1b\\";
pub(crate) const VSCODE_PRE_EXECUTION_MARKER: &str = "\x1b]633;C\x1b\\";
//"\x1b]633;D;{}\x1b\\"
pub(crate) const VSCODE_POST_EXECUTION_MARKER_PREFIX: &str = "\x1b]633;D;";
pub(crate) const VSCODE_POST_EXECUTION_MARKER_SUFFIX: &str = "\x1b\\";
//"\x1b]633;E;{}\x1b\\"
pub(crate) const VSCODE_COMMANDLINE_MARKER_PREFIX: &str = "\x1b]633;E;";
pub(crate) const VSCODE_COMMANDLINE_MARKER_SUFFIX: &str = "\x1b\\";
// "\x1b]633;P;Cwd={}\x1b\\"
pub(crate) const VSCODE_CWD_PROPERTY_MARKER_PREFIX: &str = "\x1b]633;P;Cwd=";
pub(crate) const VSCODE_CWD_PROPERTY_MARKER_SUFFIX: &str = "\x1b\\";

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
                        report_shell_error(engine_state, &err);
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

pub(crate) fn update_prompt(
    config: &Config,
    engine_state: &EngineState,
    stack: &mut Stack,
    nu_prompt: &mut NushellPrompt,
) {
    let configured_left_prompt_string =
        match get_prompt_string(PROMPT_COMMAND, config, engine_state, stack) {
            Some(s) => s,
            None => "".to_string(),
        };

    // Now that we have the prompt string lets ansify it.
    // <133 A><prompt><133 B><command><133 C><command output>
    let left_prompt_string = if config.shell_integration.osc633 {
        if stack
            .get_env_var(engine_state, "TERM_PROGRAM")
            .and_then(|v| v.as_str().ok())
            == Some("vscode")
        {
            // We're in vscode and we have osc633 enabled
            Some(format!(
                "{VSCODE_PRE_PROMPT_MARKER}{configured_left_prompt_string}{VSCODE_POST_PROMPT_MARKER}"
            ))
        } else if config.shell_integration.osc133 {
            // If we're in VSCode but we don't find the env var, but we have osc133 set, then use it
            Some(format!(
                "{PRE_PROMPT_MARKER}{configured_left_prompt_string}{POST_PROMPT_MARKER}"
            ))
        } else {
            configured_left_prompt_string.into()
        }
    } else if config.shell_integration.osc133 {
        Some(format!(
            "{PRE_PROMPT_MARKER}{configured_left_prompt_string}{POST_PROMPT_MARKER}"
        ))
    } else {
        configured_left_prompt_string.into()
    };

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
