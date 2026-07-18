use nu_protocol::engine::{PromptContents, PromptState};
#[cfg(windows)]
use nu_utils::enable_vt_processing;
use reedline::{
    DefaultPrompt, Prompt, PromptEditMode, PromptHistorySearch, PromptHistorySearchStatus,
    PromptViMode,
};
use std::{borrow::Cow, sync::Arc};

/// The reedline-facing view over the shared [`PromptState`].
///
/// This holds no prompt content of its own: every render reads the current
/// [`PromptContents`] through the shared handle, so text a background job pushes
/// via `commandline set-prompt` is picked up the next time reedline redraws. A
/// transient prompt is simply a `NushellPrompt` over a private, detached
/// `PromptState` that never receives async pushes.
pub struct NushellPrompt {
    pub state: Arc<PromptState>,
}

/// Render `content` for the terminal, or fall back to reedline's default via
/// `default` when nothing has been set. reedline needs `\r\n` line breaks.
fn render_or<'a>(content: Option<&str>, default: impl FnOnce() -> Cow<'a, str>) -> Cow<'a, str> {
    const NEWLINE: char = '\n';
    const LINEBREAK: &'static str = "\r\n";

    match content {
        Some(content) => content.replace(NEWLINE, LINEBREAK).into(),
        None => default().replace(NEWLINE, LINEBREAK).into(),
    }
}

impl Prompt for NushellPrompt {
    fn render_prompt_left(&self) -> Cow<'_, str> {
        #[cfg(windows)]
        {
            let _ = enable_vt_processing();
        }

        self.state.with_contents(|c| {
            render_or(c.left.as_deref(), || {
                DefaultPrompt::default()
                    .render_prompt_left()
                    .into_owned()
                    .into()
            })
        })
    }

    fn render_prompt_right(&self) -> Cow<'_, str> {
        self.state.with_contents(|c| {
            render_or(c.right.as_deref(), || {
                DefaultPrompt::default()
                    .render_prompt_right()
                    .into_owned()
                    .into()
            })
        })
    }

    fn render_prompt_indicator(&self, edit_mode: PromptEditMode) -> Cow<'_, str> {
        self.state
            .with_contents(|c| indicator_for(c, edit_mode).to_string())
            .into()
    }

    fn render_prompt_multiline_indicator(&self) -> Cow<'_, str> {
        self.state
            .with_contents(|c| c.multiline.clone().unwrap_or_else(|| "::: ".into()))
            .into()
    }

    fn render_prompt_history_search_indicator(
        &self,
        history_search: PromptHistorySearch,
    ) -> Cow<'_, str> {
        let prefix = match history_search.status {
            PromptHistorySearchStatus::Passing => "",
            PromptHistorySearchStatus::Failing => "failing ",
        };

        Cow::Owned(format!(
            "({}reverse-search: {})",
            prefix, history_search.term
        ))
    }

    fn right_prompt_on_last_line(&self) -> bool {
        self.state.with_contents(|c| c.render_right_on_last_line)
    }
}

/// The indicator string for the given edit mode, with the built-in defaults.
fn indicator_for(contents: &PromptContents, edit_mode: PromptEditMode) -> String {
    match edit_mode {
        PromptEditMode::Default | PromptEditMode::Emacs => {
            contents.indicator.clone().unwrap_or_else(|| "> ".into())
        }
        PromptEditMode::Vi(PromptViMode::Normal) => {
            contents.vi_normal.clone().unwrap_or_else(|| "> ".into())
        }
        PromptEditMode::Vi(PromptViMode::Insert) => {
            contents.vi_insert.clone().unwrap_or_else(|| ": ".into())
        }
        PromptEditMode::Vi(PromptViMode::Visual) => {
            contents.vi_normal.clone().unwrap_or_else(|| "v ".into())
        }
        PromptEditMode::Custom(str) => format!("({str})"),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_prompt_does_not_embed_osc_markers() {
        let prompt = NushellPrompt { state: Arc::new(PromptState::new()) };
        let rendered = prompt.render_prompt_left().to_string();

        assert!(!rendered.contains("\x1b]133;"));
        assert!(!rendered.contains("\x1b]633;"));
    }
}
