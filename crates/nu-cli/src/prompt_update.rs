use crate::NushellPrompt;
use log::trace;
use nu_engine::ClosureEvalOnce;
use nu_protocol::{
    engine::{EngineState, Stack},
    report_error_new, Config, PipelineData, Value,
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
#[allow(dead_code)]
pub(crate) const POST_EXECUTION_MARKER_PREFIX: &str = "\x1b]133;D;";
#[allow(dead_code)]
pub(crate) const POST_EXECUTION_MARKER_SUFFIX: &str = "\x1b\\";

// OSC633 is the same as OSC133 but specifically for VSCode
pub(crate) const VSCODE_PRE_PROMPT_MARKER: &str = "\x1b]633;A\x1b\\";
pub(crate) const VSCODE_POST_PROMPT_MARKER: &str = "\x1b]633;B\x1b\\";
#[allow(dead_code)]
pub(crate) const VSCODE_PRE_EXECUTION_MARKER: &str = "\x1b]633;C\x1b\\";
#[allow(dead_code)]
//"\x1b]633;D;{}\x1b\\"
pub(crate) const VSCODE_POST_EXECUTION_MARKER_PREFIX: &str = "\x1b]633;D;";
#[allow(dead_code)]
pub(crate) const VSCODE_POST_EXECUTION_MARKER_SUFFIX: &str = "\x1b\\";
#[allow(dead_code)]
pub(crate) const VSCODE_COMMANDLINE_MARKER: &str = "\x1b]633;E\x1b\\";
#[allow(dead_code)]
// "\x1b]633;P;Cwd={}\x1b\\"
pub(crate) const VSCODE_CWD_PROPERTY_MARKER_PREFIX: &str = "\x1b]633;P;Cwd=";
#[allow(dead_code)]
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
                let result = ClosureEvalOnce::new(engine_state, stack, val)
                    .run_with_input(PipelineData::Empty);

                trace!(
                    "get_prompt_string (block) {}:{}:{}",
                    file!(),
                    line!(),
                    column!()
                );

                result
                    .map_err(|err| {
                        report_error_new(engine_state, &err);
                    })
                    .ok()
            }
            Value::String { .. } => Some(PipelineData::Value(v.clone(), None)),
            _ => None,
        })
        .and_then(|pipeline_data| {
            let output = pipeline_data.collect_string("", config).ok();

            output.map(|mut x| {
                // Just remove the very last newline.
                if x.ends_with('\n') {
                    x.pop();
                }

                if x.ends_with('\r') {
                    x.pop();
                }
                x
            })
        })
}

pub(crate) fn update_prompt(
    config: &Config,
    engine_state: &EngineState,
    stack: &mut Stack,
    nu_prompt: &mut NushellPrompt,
) {
    let left_prompt_string = get_prompt_string(PROMPT_COMMAND, config, engine_state, stack);

    // Now that we have the prompt string lets ansify it.
    // <133 A><prompt><133 B><command><133 C><command output>
    let left_prompt_string_133 = if config.shell_integration_osc133 {
        if let Some(prompt_string) = left_prompt_string.clone() {
            Some(format!(
                "{PRE_PROMPT_MARKER}{prompt_string}{POST_PROMPT_MARKER}"
            ))
        } else {
            left_prompt_string.clone()
        }
    } else {
        left_prompt_string.clone()
    };

    let left_prompt_string_633 = if config.shell_integration_osc633 {
        if let Some(prompt_string) = left_prompt_string.clone() {
            if stack.get_env_var(engine_state, "TERM_PROGRAM") == Some(Value::test_string("vscode"))
            {
                // If the user enabled osc633 and we're in vscode, use the vscode markers
                Some(format!(
                    "{VSCODE_PRE_PROMPT_MARKER}{prompt_string}{VSCODE_POST_PROMPT_MARKER}"
                ))
            } else {
                // otherwise, use the regular osc133 markers
                Some(format!(
                    "{PRE_PROMPT_MARKER}{prompt_string}{POST_PROMPT_MARKER}"
                ))
            }
        } else {
            left_prompt_string.clone()
        }
    } else {
        left_prompt_string.clone()
    };

    let left_prompt_string = match (left_prompt_string_133, left_prompt_string_633) {
        (None, None) => left_prompt_string,
        (None, Some(l633)) => Some(l633),
        (Some(l133), None) => Some(l133),
        // If both are set, it means we're in vscode, so use the vscode markers
        // and even if we're not actually in vscode atm, the regular 133 markers are used
        (Some(_l133), Some(l633)) => Some(l633),
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
