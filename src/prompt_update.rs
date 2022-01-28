use nu_cli::NushellPrompt;
use nu_engine::eval_subexpression;
use nu_parser::parse;
use nu_protocol::{
    engine::{EngineState, Stack, StateWorkingSet},
    Config, PipelineData, Span, Value,
};
use reedline::Prompt;

// Name of environment variable where the prompt could be stored
pub(crate) const PROMPT_COMMAND: &str = "PROMPT_COMMAND";
pub(crate) const PROMPT_COMMAND_RIGHT: &str = "PROMPT_COMMAND_RIGHT";
pub(crate) const PROMPT_INDICATOR: &str = "PROMPT_INDICATOR";
pub(crate) const PROMPT_INDICATOR_VI_INSERT: &str = "PROMPT_INDICATOR_VI_INSERT";
pub(crate) const PROMPT_INDICATOR_VI_NORMAL: &str = "PROMPT_INDICATOR_VI_NORMAL";
pub(crate) const PROMPT_MULTILINE_INDICATOR: &str = "PROMPT_MULTILINE_INDICATOR";

pub(crate) fn get_prompt_indicators(
    config: &Config,
    engine_state: &EngineState,
    stack: &Stack,
) -> (String, String, String, String) {
    let prompt_indicator = match stack.get_env_var(engine_state, PROMPT_INDICATOR) {
        Some(pi) => pi.into_string("", config),
        None => "〉".to_string(),
    };

    let prompt_vi_insert = match stack.get_env_var(engine_state, PROMPT_INDICATOR_VI_INSERT) {
        Some(pvii) => pvii.into_string("", config),
        None => ": ".to_string(),
    };

    let prompt_vi_normal = match stack.get_env_var(engine_state, PROMPT_INDICATOR_VI_NORMAL) {
        Some(pviv) => pviv.into_string("", config),
        None => "〉".to_string(),
    };

    let prompt_multiline = match stack.get_env_var(engine_state, PROMPT_MULTILINE_INDICATOR) {
        Some(pm) => pm.into_string("", config),
        None => "::: ".to_string(),
    };

    (
        prompt_indicator,
        prompt_vi_insert,
        prompt_vi_normal,
        prompt_multiline,
    )
}

fn get_prompt_string(
    prompt: &str,
    config: &Config,
    engine_state: &EngineState,
    stack: &mut Stack,
) -> Option<String> {
    stack
        .get_env_var(engine_state, prompt)
        .and_then(|v| match v {
            Value::Block { val: block_id, .. } => {
                let block = engine_state.get_block(block_id);
                // Use eval_subexpression to force a redirection of output, so we can use everything in prompt
                eval_subexpression(
                    engine_state,
                    stack,
                    block,
                    PipelineData::new(Span::new(0, 0)), // Don't try this at home, 0 span is ignored
                )
                .ok()
            }
            Value::String { val: source, .. } => {
                let mut working_set = StateWorkingSet::new(engine_state);
                let (block, _) = parse(&mut working_set, None, source.as_bytes(), true);
                // Use eval_subexpression to force a redirection of output, so we can use everything in prompt
                eval_subexpression(
                    engine_state,
                    stack,
                    &block,
                    PipelineData::new(Span::new(0, 0)), // Don't try this at home, 0 span is ignored
                )
                .ok()
            }
            _ => None,
        })
        .and_then(|pipeline_data| {
            let output = pipeline_data.collect_string("", config).ok();

            match output {
                Some(mut x) => {
                    // Just remove the very last newline.
                    if x.ends_with('\n') {
                        x.pop();
                    }

                    if x.ends_with('\r') {
                        x.pop();
                    }
                    Some(x)
                }
                None => None,
            }
        })
}

pub(crate) fn update_prompt<'prompt>(
    config: &Config,
    engine_state: &EngineState,
    stack: &Stack,
    nu_prompt: &'prompt mut NushellPrompt,
) -> &'prompt dyn Prompt {
    // get the other indicators
    let (
        prompt_indicator_string,
        prompt_vi_insert_string,
        prompt_vi_normal_string,
        prompt_multiline_string,
    ) = get_prompt_indicators(config, engine_state, stack);

    let mut stack = stack.clone();

    // apply the other indicators
    nu_prompt.update_all_prompt_strings(
        get_prompt_string(PROMPT_COMMAND, config, engine_state, &mut stack),
        get_prompt_string(PROMPT_COMMAND_RIGHT, config, engine_state, &mut stack),
        prompt_indicator_string,
        prompt_multiline_string,
        (prompt_vi_insert_string, prompt_vi_normal_string),
    );

    nu_prompt as &dyn Prompt
}
