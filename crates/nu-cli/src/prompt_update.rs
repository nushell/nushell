use crate::NushellPrompt;
use log::trace;
use nu_engine::ClosureEvalOnce;
use nu_protocol::{
    engine::{EngineState, Stack, StateWorkingSet},
    report_error, Config, PipelineData, Value,
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
// According to Daniel Imms @Tyriar, we need to do these this way:
// <133 A><prompt><133 B><command><133 C><command output>
pub(crate) const PRE_PROMPT_MARKER: &str = "\x1b]133;A\x1b\\";
pub(crate) const POST_PROMPT_MARKER: &str = "\x1b]133;B\x1b\\";

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
                        let working_set = StateWorkingSet::new(engine_state);
                        report_error(&working_set, &err);
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
    let left_prompt_string = if config.shell_integration {
        if let Some(prompt_string) = left_prompt_string {
            Some(format!(
                "{PRE_PROMPT_MARKER}{prompt_string}{POST_PROMPT_MARKER}"
            ))
        } else {
            left_prompt_string
        }
    } else {
        left_prompt_string
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
