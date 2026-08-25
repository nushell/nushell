use nu_cmd_base::prompt::{
    PROMPT_INDICATOR_VI_INSERT, PROMPT_INDICATOR_VI_NORMAL, PROMPT_MULTILINE_INDICATOR,
    resolve_indicator,
};
use nu_protocol::{
    Config,
    engine::{EngineState, Stack},
};
use reedline::PromptHelixMode;
use reedline::{
    Prompt, PromptEditMode, PromptHistorySearch, PromptHistorySearchStatus, PromptViMode,
};

use std::borrow::Cow;

/// The resolved mode indicators `input --reedline` draws. Only these four: the
/// left prompt is the caller's own text, and `input` has no transient prompt.
#[derive(Clone)]
pub struct ModeIndicators {
    pub vi_normal: String,
    pub vi_insert: String,
    pub vi_visual: String,
    pub multiline: String,
}

impl ModeIndicators {
    /// Resolved the way the REPL does, so a user still on the environment
    /// variables sees the same indicators in both places.
    ///
    /// TODO: drop the variable lookups once the `$env.PROMPT_*` deprecation
    /// closes; this can then read `config.prompt` directly.
    pub fn resolve(config: &Config, engine_state: &EngineState, stack: &Stack) -> Self {
        let indicator = |env_var, configured: &str| {
            resolve_indicator(env_var, configured, config, engine_state, stack).into_owned()
        };

        Self {
            vi_normal: indicator(PROMPT_INDICATOR_VI_NORMAL, &config.prompt.vi_normal),
            vi_insert: indicator(PROMPT_INDICATOR_VI_INSERT, &config.prompt.vi_insert),
            // Config-only: vi visual mode never had a variable of its own.
            vi_visual: config.prompt.vi_visual.clone(),
            multiline: indicator(PROMPT_MULTILINE_INDICATOR, &config.prompt.multiline),
        }
    }
}

/// Simple [`Prompt`] displaying a configurable left and a right prompt.
/// For more fine-tuned configuration, implement the [`Prompt`] trait.
/// For the default configuration, use [`DefaultPrompt::default()`]
#[derive(Clone)]
pub struct ReedlinePrompt {
    /// What segment should be rendered in the left (main) prompt
    pub left_prompt: String,
    pub right_prompt: String,
    /// Rendered in the indicator slot outside of vi mode. `input` repurposes it
    /// for the `(default: ...)` hint, so it is empty unless the caller passed
    /// both a prompt and a default. `$env.config.prompt.indicator` deliberately
    /// does not apply: appending `> ` to the caller's own prompt text would
    /// render a second prompt.
    pub indicator: String,
    pub indicators: ModeIndicators,
}

impl Prompt for ReedlinePrompt {
    fn render_prompt_left(&self) -> Cow<'_, str> {
        Cow::Borrowed(&self.left_prompt)
    }

    fn render_prompt_right(&self) -> Cow<'_, str> {
        Cow::Borrowed(&self.right_prompt)
    }

    fn render_prompt_indicator(&self, edit_mode: PromptEditMode) -> Cow<'_, str> {
        match edit_mode {
            PromptEditMode::Default | PromptEditMode::Emacs => self.indicator.as_str().into(),
            PromptEditMode::Vi(vi_mode) => match vi_mode {
                PromptViMode::Normal => self.indicators.vi_normal.as_str().into(),
                PromptViMode::Insert => self.indicators.vi_insert.as_str().into(),
                PromptViMode::Visual => self.indicators.vi_visual.as_str().into(),
            },
            PromptEditMode::Helix(helix_mode) => match helix_mode {
                PromptHelixMode::Normal | PromptHelixMode::Select => {
                    DEFAULT_VI_NORMAL_PROMPT_INDICATOR.into()
                }
                PromptHelixMode::Insert => DEFAULT_VI_INSERT_PROMPT_INDICATOR.into(),
            },
            PromptEditMode::Custom(str) => format!("({str})").into(),
        }
    }

    fn render_prompt_multiline_indicator(&self) -> Cow<'_, str> {
        self.indicators.multiline.as_str().into()
    }

    fn render_prompt_history_search_indicator(
        &self,
        history_search: PromptHistorySearch,
    ) -> Cow<'_, str> {
        let prefix = match history_search.status {
            PromptHistorySearchStatus::Passing => "",
            PromptHistorySearchStatus::Failing => "failing ",
        };
        // NOTE: magic strings, given there is logic on how these compose I am not sure if it
        // is worth extracting in to static constant
        Cow::Owned(format!(
            "({}reverse-search: {}) ",
            prefix, history_search.term
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use nu_protocol::{Span, Value};

    /// Resolves the indicators from `config`, with `env` applied first.
    fn resolved(config: &Config, env: &[(&str, &str)]) -> ModeIndicators {
        let engine_state = EngineState::new();
        let mut stack = Stack::new();
        for (name, val) in env {
            stack.add_env_var((*name).into(), Value::string(*val, Span::test_data()));
        }

        ModeIndicators::resolve(config, &engine_state, &stack)
    }

    #[test]
    fn indicators_come_from_config_without_env_var() {
        let mut config = Config::default();
        config.prompt.vi_normal = "config> ".into();

        assert_eq!(resolved(&config, &[]).vi_normal, "config> ");
    }

    #[test]
    fn legacy_env_var_takes_precedence_over_config() {
        // Matches the REPL, so a user still on the variables does not get one
        // indicator at the prompt and a different one inside `input`.
        let mut config = Config::default();
        config.prompt.vi_normal = "config> ".into();

        assert_eq!(
            resolved(&config, &[(PROMPT_INDICATOR_VI_NORMAL, "env> ")]).vi_normal,
            "env> "
        );
    }
}
