use nu_cmd_base::prompt;
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
    /// Read from `$env.config.prompt` alone. `input` never honored the
    /// deprecated `$env.PROMPT_*` variables, and outside the REPL the only
    /// copies it could find are the ones a parent shell exported.
    ///
    /// A closure is evaluated once, here; one that fails reports its error and
    /// draws nothing.
    pub fn from_config(config: &Config, engine_state: &EngineState, stack: &Stack) -> Self {
        let prompt = &config.prompt;
        let render = |value| prompt::render(value, config, engine_state, stack).unwrap_or_default();
        Self {
            vi_normal: render(&prompt.vi_normal),
            vi_insert: render(&prompt.vi_insert),
            vi_visual: render(&prompt.vi_visual),
            multiline: render(&prompt.multiline),
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
            // The config keys are named for vi but the state is what they
            // describe, so helix reads the same three. Select takes the visual
            // key rather than normal's, matching reedline's own default.
            PromptEditMode::Helix(helix_mode) => match helix_mode {
                PromptHelixMode::Normal => self.indicators.vi_normal.as_str().into(),
                PromptHelixMode::Select => self.indicators.vi_visual.as_str().into(),
                PromptHelixMode::Insert => self.indicators.vi_insert.as_str().into(),
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
    use nu_protocol::Value;

    #[test]
    fn indicators_come_from_their_own_config_keys() {
        let mut config = Config::default();
        config.prompt.vi_normal = Value::test_string("normal> ");
        config.prompt.vi_insert = Value::test_string("insert> ");
        config.prompt.vi_visual = Value::test_string("visual> ");
        config.prompt.multiline = Value::test_string("... ");

        let indicators = ModeIndicators::from_config(&config, &EngineState::new(), &Stack::new());
        assert_eq!(
            [
                indicators.vi_normal,
                indicators.vi_insert,
                indicators.vi_visual,
                indicators.multiline,
            ],
            ["normal> ", "insert> ", "visual> ", "... "],
        );
    }
}
