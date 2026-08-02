use nu_protocol::engine::{PromptContents, PromptState};
#[cfg(windows)]
use nu_utils::enable_vt_processing;
use reedline::{
    DefaultPrompt, Prompt, PromptEditMode, PromptHistorySearch, PromptHistorySearchStatus,
    PromptViMode,
};
use std::{borrow::Cow, sync::Arc};

/// The reedline-facing view over some [`PromptContents`].
pub struct NushellPrompt {
    source: PromptSource,
}

/// Where a [`NushellPrompt`] reads its contents from.
enum PromptSource {
    /// The live, interactive prompt, shared with every background job.
    Shared(Arc<PromptState>),

    /// A one-off, detached copy used for the transient prompt.
    Snapshot(PromptContents),
}

impl NushellPrompt {
    /// A live prompt backed by the engine's shared [`PromptState`].
    pub fn shared(state: Arc<PromptState>) -> Self {
        Self {
            source: PromptSource::Shared(state),
        }
    }

    /// A detached prompt over a fixed snapshot (e.g. the transient prompt).
    pub fn snapshot(contents: PromptContents) -> Self {
        Self {
            source: PromptSource::Snapshot(contents),
        }
    }

    /// Read the current contents, taking the lock only for the shared variant.
    fn with_contents<R>(&self, action: impl FnOnce(&PromptContents) -> R) -> R {
        match &self.source {
            PromptSource::Shared(state) => state.with_contents(action),
            PromptSource::Snapshot(contents) => action(contents),
        }
    }
}

/// Render `content` for the terminal, or fall back to reedline's default via
/// `default` when nothing has been set. reedline needs `\r\n` line breaks.
fn render_or<'a>(content: Option<&str>, default: impl FnOnce() -> Cow<'a, str>) -> Cow<'a, str> {
    const NEWLINE: char = '\n';
    const LINEBREAK: &str = "\r\n";

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

        self.with_contents(|c| {
            render_or(c.left.as_deref(), || {
                DefaultPrompt::default()
                    .render_prompt_left()
                    .into_owned()
                    .into()
            })
        })
    }

    fn render_prompt_right(&self) -> Cow<'_, str> {
        self.with_contents(|c| {
            render_or(c.right.as_deref(), || {
                DefaultPrompt::default()
                    .render_prompt_right()
                    .into_owned()
                    .into()
            })
        })
    }

    fn render_prompt_indicator(&self, edit_mode: PromptEditMode) -> Cow<'_, str> {
        self.with_contents(|c| indicator_for(c, edit_mode)).into()
    }

    fn render_prompt_multiline_indicator(&self) -> Cow<'_, str> {
        self.with_contents(|c| c.multiline.as_deref().unwrap_or("::: ").to_string())
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
        self.with_contents(|c| c.render_right_on_last_line)
    }
}

/// The indicator string for the given edit mode, with the built-in defaults.
fn indicator_for(contents: &PromptContents, edit_mode: PromptEditMode) -> String {
    match edit_mode {
        PromptEditMode::Default | PromptEditMode::Emacs => {
            contents.indicator.as_deref().unwrap_or("> ").to_string()
        }
        PromptEditMode::Vi(PromptViMode::Normal) => {
            contents.vi_normal.as_deref().unwrap_or("> ").to_string()
        }
        PromptEditMode::Vi(PromptViMode::Insert) => {
            contents.vi_insert.as_deref().unwrap_or(": ").to_string()
        }
        PromptEditMode::Vi(PromptViMode::Visual) => {
            contents.vi_normal.as_deref().unwrap_or("v ").to_string()
        }
        PromptEditMode::Custom(str) => format!("({str})"),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_prompt_does_not_embed_osc_markers() {
        let prompt = NushellPrompt::shared(Arc::new(PromptState::new()));
        let rendered = prompt.render_prompt_left().to_string();

        assert!(!rendered.contains("\x1b]133;"));
        assert!(!rendered.contains("\x1b]633;"));
    }
}
