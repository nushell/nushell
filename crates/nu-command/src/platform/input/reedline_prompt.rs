#[cfg(feature = "helix")]
use reedline::PromptHelixMode;
use reedline::{
    Prompt, PromptEditMode, PromptHistorySearch, PromptHistorySearchStatus, PromptViMode,
};

use std::borrow::Cow;

/// The default prompt indicator
pub static DEFAULT_VI_INSERT_PROMPT_INDICATOR: &str = ": ";
pub static DEFAULT_VI_NORMAL_PROMPT_INDICATOR: &str = "〉";
pub static DEFAULT_MULTILINE_INDICATOR: &str = "::: ";

/// Simple [`Prompt`] displaying a configurable left and a right prompt.
/// For more fine-tuned configuration, implement the [`Prompt`] trait.
/// For the default configuration, use [`DefaultPrompt::default()`]
#[derive(Clone)]
pub struct ReedlinePrompt {
    /// What segment should be rendered in the left (main) prompt
    pub left_prompt: String,
    pub right_prompt: String,
    pub indicator: String,
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
                PromptViMode::Normal => DEFAULT_VI_NORMAL_PROMPT_INDICATOR.into(),
                PromptViMode::Insert => DEFAULT_VI_INSERT_PROMPT_INDICATOR.into(),
                PromptViMode::Visual => DEFAULT_VI_NORMAL_PROMPT_INDICATOR.into(),
            },
            #[cfg(feature = "helix")]
            PromptEditMode::Helix(helix_mode) => match helix_mode {
                PromptHelixMode::Normal | PromptHelixMode::Select => {
                    DEFAULT_VI_NORMAL_PROMPT_INDICATOR.into()
                }
                PromptHelixMode::Insert => DEFAULT_VI_INSERT_PROMPT_INDICATOR.into(),
            },
            PromptEditMode::Custom(str) => format!("({str})").into(),
            // Reedline's `helix` feature can be switched on by another crate in
            // the build graph without this crate's `helix` feature following
            // along; catch the then-uncovered variants instead of failing to
            // compile.
            #[allow(unreachable_patterns)]
            _ => self.indicator.as_str().into(),
        }
    }

    fn render_prompt_multiline_indicator(&self) -> Cow<'_, str> {
        Cow::Borrowed(DEFAULT_MULTILINE_INDICATOR)
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
