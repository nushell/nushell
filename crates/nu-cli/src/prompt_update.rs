use crate::NushellPrompt;
use log::trace;
use nu_engine::eval_subexpression;
use nu_protocol::report_error;
use nu_protocol::{
    engine::{EngineState, Stack, StateWorkingSet},
    Config, PipelineData, Value,
};
use reedline::Prompt;
use std::borrow::Cow;
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
// According to Daniel Imms @Tyriar, we need to do these this way:
// <133 A><prompt><133 B><command><133 C><command output>
const PRE_PROMPT_MARKER: &str = "\x1b]133;A\x1b\\";
const POST_PROMPT_MARKER: &str = "\x1b]133;B\x1b\\";

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
                let block = engine_state.get_block(val.block_id);
                let mut stack = stack.captures_to_stack(val.captures);
                // Use eval_subexpression to force a redirection of output, so we can use everything in prompt
                let ret_val =
                    eval_subexpression(engine_state, &mut stack, block, PipelineData::empty());
                trace!(
                    "get_prompt_string (block) {}:{}:{}",
                    file!(),
                    line!(),
                    column!()
                );

                ret_val
                    .map_err(|err| {
                        let working_set = StateWorkingSet::new(engine_state);
                        report_error(&working_set, &err);
                    })
                    .ok()
            }
            Value::Block { val: block_id, .. } => {
                let block = engine_state.get_block(block_id);
                // Use eval_subexpression to force a redirection of output, so we can use everything in prompt
                let ret_val = eval_subexpression(engine_state, stack, block, PipelineData::empty());
                trace!(
                    "get_prompt_string (block) {}:{}:{}",
                    file!(),
                    line!(),
                    column!()
                );

                ret_val
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

pub(crate) fn update_prompt<'prompt>(
    config: &Config,
    engine_state: &EngineState,
    stack: &Stack,
    nu_prompt: &'prompt mut NushellPrompt,
) -> &'prompt dyn Prompt {
    let mut stack = stack.clone();

    let left_prompt_string = get_prompt_string(PROMPT_COMMAND, config, engine_state, &mut stack);

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

    let right_prompt_string =
        get_prompt_string(PROMPT_COMMAND_RIGHT, config, engine_state, &mut stack);

    let prompt_indicator_string =
        get_prompt_string(PROMPT_INDICATOR, config, engine_state, &mut stack);

    let prompt_multiline_string =
        get_prompt_string(PROMPT_MULTILINE_INDICATOR, config, engine_state, &mut stack);

    let prompt_vi_insert_string =
        get_prompt_string(PROMPT_INDICATOR_VI_INSERT, config, engine_state, &mut stack);

    let prompt_vi_normal_string =
        get_prompt_string(PROMPT_INDICATOR_VI_NORMAL, config, engine_state, &mut stack);

    // apply the other indicators
    nu_prompt.update_all_prompt_strings(
        left_prompt_string,
        right_prompt_string,
        prompt_indicator_string,
        prompt_multiline_string,
        (prompt_vi_insert_string, prompt_vi_normal_string),
        config.render_right_prompt_on_last_line,
    );

    let ret_val = nu_prompt as &dyn Prompt;
    trace!("update_prompt {}:{}:{}", file!(), line!(), column!());

    ret_val
}

struct TransientPrompt {
    engine_state: Arc<EngineState>,
    stack: Stack,
}

/// Try getting `$env.TRANSIENT_PROMPT_<X>`, and get `$env.PROMPT_<X>` if that fails
fn get_transient_prompt_string(
    transient_prompt: &str,
    prompt: &str,
    config: &Config,
    engine_state: &EngineState,
    stack: &mut Stack,
) -> Option<String> {
    get_prompt_string(transient_prompt, config, engine_state, stack)
        .or_else(|| get_prompt_string(prompt, config, engine_state, stack))
}

impl Prompt for TransientPrompt {
    fn render_prompt_left(&self) -> Cow<str> {
        let mut nu_prompt = NushellPrompt::new();
        let config = &self.engine_state.get_config().clone();
        let mut stack = self.stack.clone();
        nu_prompt.update_prompt_left(get_transient_prompt_string(
            TRANSIENT_PROMPT_COMMAND,
            PROMPT_COMMAND,
            config,
            &self.engine_state,
            &mut stack,
        ));
        nu_prompt.render_prompt_left().to_string().into()
    }

    fn render_prompt_right(&self) -> Cow<str> {
        let mut nu_prompt = NushellPrompt::new();
        let config = &self.engine_state.get_config().clone();
        let mut stack = self.stack.clone();
        nu_prompt.update_prompt_right(
            get_transient_prompt_string(
                TRANSIENT_PROMPT_COMMAND_RIGHT,
                PROMPT_COMMAND_RIGHT,
                config,
                &self.engine_state,
                &mut stack,
            ),
            config.render_right_prompt_on_last_line,
        );
        nu_prompt.render_prompt_right().to_string().into()
    }

    fn render_prompt_indicator(&self, prompt_mode: reedline::PromptEditMode) -> Cow<str> {
        let mut nu_prompt = NushellPrompt::new();
        let config = &self.engine_state.get_config().clone();
        let mut stack = self.stack.clone();
        nu_prompt.update_prompt_indicator(get_transient_prompt_string(
            TRANSIENT_PROMPT_INDICATOR,
            PROMPT_INDICATOR,
            config,
            &self.engine_state,
            &mut stack,
        ));
        nu_prompt.update_prompt_vi_insert(get_transient_prompt_string(
            TRANSIENT_PROMPT_INDICATOR_VI_INSERT,
            PROMPT_INDICATOR_VI_INSERT,
            config,
            &self.engine_state,
            &mut stack,
        ));
        nu_prompt.update_prompt_vi_normal(get_transient_prompt_string(
            TRANSIENT_PROMPT_INDICATOR_VI_NORMAL,
            PROMPT_INDICATOR_VI_NORMAL,
            config,
            &self.engine_state,
            &mut stack,
        ));
        nu_prompt
            .render_prompt_indicator(prompt_mode)
            .to_string()
            .into()
    }

    fn render_prompt_multiline_indicator(&self) -> Cow<str> {
        let mut nu_prompt = NushellPrompt::new();
        let config = &self.engine_state.get_config().clone();
        let mut stack = self.stack.clone();
        nu_prompt.update_prompt_multiline(get_transient_prompt_string(
            TRANSIENT_PROMPT_MULTILINE_INDICATOR,
            PROMPT_MULTILINE_INDICATOR,
            config,
            &self.engine_state,
            &mut stack,
        ));
        nu_prompt
            .render_prompt_multiline_indicator()
            .to_string()
            .into()
    }

    fn render_prompt_history_search_indicator(
        &self,
        history_search: reedline::PromptHistorySearch,
    ) -> Cow<str> {
        NushellPrompt::new()
            .render_prompt_history_search_indicator(history_search)
            .to_string()
            .into()
    }
}

/// Construct the transient prompt
pub(crate) fn transient_prompt(engine_state: Arc<EngineState>, stack: &Stack) -> Box<dyn Prompt> {
    Box::new(TransientPrompt {
        engine_state,
        stack: stack.clone(),
    })
}
