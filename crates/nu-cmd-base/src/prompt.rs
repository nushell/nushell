//! Resolving prompt segments from `$env.config.prompt` and the deprecated
//! `$env.PROMPT_*` variables that still override it.
//!
//! Shared so the REPL and `input --reedline` cannot drift on precedence.

use log::{info, trace};
use nu_engine::ClosureEvalOnce;
use nu_protocol::{
    Config, PipelineData, Value,
    engine::{EngineState, Stack},
    report_shell_error,
};
use std::borrow::Cow;

/// The deprecated variables, in the order their config keys are declared.
pub const PROMPT_COMMAND: &str = "PROMPT_COMMAND";
pub const PROMPT_COMMAND_RIGHT: &str = "PROMPT_COMMAND_RIGHT";
pub const PROMPT_INDICATOR: &str = "PROMPT_INDICATOR";
pub const PROMPT_INDICATOR_VI_INSERT: &str = "PROMPT_INDICATOR_VI_INSERT";
pub const PROMPT_INDICATOR_VI_NORMAL: &str = "PROMPT_INDICATOR_VI_NORMAL";
pub const PROMPT_MULTILINE_INDICATOR: &str = "PROMPT_MULTILINE_INDICATOR";
pub const TRANSIENT_PROMPT_COMMAND: &str = "TRANSIENT_PROMPT_COMMAND";
pub const TRANSIENT_PROMPT_COMMAND_RIGHT: &str = "TRANSIENT_PROMPT_COMMAND_RIGHT";
pub const TRANSIENT_PROMPT_INDICATOR: &str = "TRANSIENT_PROMPT_INDICATOR";
pub const TRANSIENT_PROMPT_INDICATOR_VI_INSERT: &str = "TRANSIENT_PROMPT_INDICATOR_VI_INSERT";
pub const TRANSIENT_PROMPT_INDICATOR_VI_NORMAL: &str = "TRANSIENT_PROMPT_INDICATOR_VI_NORMAL";
pub const TRANSIENT_PROMPT_MULTILINE_INDICATOR: &str = "TRANSIENT_PROMPT_MULTILINE_INDICATOR";

/// A string is used verbatim, a closure is evaluated once. Anything else has
/// no rendering.
fn render(
    value: &Value,
    config: &Config,
    engine_state: &EngineState,
    stack: &Stack,
) -> Option<String> {
    let output = match value {
        Value::String { val, .. } => val.clone(),
        Value::Closure { val, .. } => {
            let result = ClosureEvalOnce::new(engine_state, stack, val.as_ref().clone())
                .run_with_input(PipelineData::empty());

            trace!("render (block) {}:{}:{}", file!(), line!(), column!());

            result
                .map_err(|err| report_shell_error(None, engine_state, &err))
                .ok()
                .and_then(|pd| pd.collect_string("", config).ok())?
        }
        _ => return None,
    };

    // Let's keep this for debugging purposes with nu --log-level warn
    info!("{}:{}:{} {:?}", file!(), line!(), column!(), output);

    Some(output)
}

/// A variable that is set but fails to render yields `None`, so every caller
/// treats a broken value as absent rather than blanking the segment.
fn from_env(
    env_var: &str,
    config: &Config,
    engine_state: &EngineState,
    stack: &Stack,
) -> Option<String> {
    render(
        stack.get_env_var(engine_state, env_var)?,
        config,
        engine_state,
        stack,
    )
}

/// The prompt itself. `None` from both sources leaves the line editor's own.
pub fn resolve_prompt_source(
    env_var: &str,
    configured: Option<&Value>,
    config: &Config,
    engine_state: &EngineState,
    stack: &Stack,
) -> Option<String> {
    from_env(env_var, config, engine_state, stack)
        .or_else(|| render(configured?, config, engine_state, stack))
}

/// A mode indicator, which unlike the prompt always has a configured string
/// to fall back on. Borrowed when nothing overrode it, the usual case.
pub fn resolve_indicator<'a>(
    env_var: &str,
    configured: &'a str,
    config: &Config,
    engine_state: &EngineState,
    stack: &Stack,
) -> Cow<'a, str> {
    match from_env(env_var, config, engine_state, stack) {
        Some(rendered) => Cow::Owned(rendered),
        None => Cow::Borrowed(configured),
    }
}

/// A transient indicator, where `None` from both sources means "keep whatever
/// the live prompt showed" rather than "use a default".
pub fn resolve_transient_indicator<'a>(
    env_var: &str,
    configured: Option<&'a str>,
    config: &Config,
    engine_state: &EngineState,
    stack: &Stack,
) -> Option<Cow<'a, str>> {
    from_env(env_var, config, engine_state, stack)
        .map(Cow::Owned)
        .or_else(|| configured.map(Cow::Borrowed))
}
