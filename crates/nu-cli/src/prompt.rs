#[cfg(windows)]
use nu_utils::enable_vt_processing;
use reedline::{
    DefaultPrompt, Prompt, PromptEditMode, PromptHistorySearch, PromptHistorySearchStatus,
    PromptViMode,
};
use std::borrow::Cow;

/// Nushell prompt definition
#[derive(Default, Clone)]
pub struct NushellPrompt {
    left_prompt: Option<String>,
    right_prompt: Option<String>,
    prompt_indicator: Option<String>,
    vi_insert_prompt_indicator: Option<String>,
    vi_normal_prompt_indicator: Option<String>,
    multiline_indicator: Option<String>,
    render_right_prompt_on_last_line: bool,
}

impl NushellPrompt {
    pub fn new() -> NushellPrompt {
        NushellPrompt {
            left_prompt: None,
            right_prompt: None,
            prompt_indicator: None,
            vi_insert_prompt_indicator: None,
            vi_normal_prompt_indicator: None,
            multiline_indicator: None,
            render_right_prompt_on_last_line: false,
        }
    }

    pub fn update_prompt_left(&mut self, left_prompt_string: Option<String>) {
        self.left_prompt = left_prompt_string;
    }

    pub fn update_prompt_right(
        &mut self,
        right_prompt_string: Option<String>,
        render_right_prompt_on_last_line: bool,
    ) {
        self.right_prompt = right_prompt_string;
        self.render_right_prompt_on_last_line = render_right_prompt_on_last_line;
    }

    pub fn update_prompt_indicator(&mut self, prompt_indicator_string: Option<String>) {
        self.prompt_indicator = prompt_indicator_string;
    }

    pub fn update_prompt_vi_insert(&mut self, prompt_vi_insert_string: Option<String>) {
        self.vi_insert_prompt_indicator = prompt_vi_insert_string;
    }

    pub fn update_prompt_vi_normal(&mut self, prompt_vi_normal_string: Option<String>) {
        self.vi_normal_prompt_indicator = prompt_vi_normal_string;
    }

    pub fn update_prompt_multiline(&mut self, prompt_multiline_indicator_string: Option<String>) {
        self.multiline_indicator = prompt_multiline_indicator_string;
    }

    pub fn update_all_prompt_strings(
        &mut self,
        left_prompt_string: Option<String>,
        right_prompt_string: Option<String>,
        prompt_indicator_string: Option<String>,
        prompt_multiline_indicator_string: Option<String>,
        prompt_vi: (Option<String>, Option<String>),
        render_right_prompt_on_last_line: bool,
    ) {
        let (prompt_vi_insert_string, prompt_vi_normal_string) = prompt_vi;

        self.left_prompt = left_prompt_string;
        self.right_prompt = right_prompt_string;
        self.prompt_indicator = prompt_indicator_string;
        self.multiline_indicator = prompt_multiline_indicator_string;
        self.vi_insert_prompt_indicator = prompt_vi_insert_string;
        self.vi_normal_prompt_indicator = prompt_vi_normal_string;
        self.render_right_prompt_on_last_line = render_right_prompt_on_last_line;
    }

    fn default_wrapped_custom_string(&self, str: String) -> String {
        format!("({str})")
    }
}

impl Prompt for NushellPrompt {
    fn render_prompt_left(&self) -> Cow<'_, str> {
        #[cfg(windows)]
        {
            let _ = enable_vt_processing();
        }

        if let Some(prompt_string) = &self.left_prompt {
            prompt_string.replace('\n', "\r\n").into()
        } else {
            let default = DefaultPrompt::default();
            default
                .render_prompt_left()
                .to_string()
                .replace('\n', "\r\n")
                .into()
        }
    }

    fn render_prompt_right(&self) -> Cow<'_, str> {
        if let Some(prompt_string) = &self.right_prompt {
            prompt_string.replace('\n', "\r\n").into()
        } else {
            let default = DefaultPrompt::default();
            default
                .render_prompt_right()
                .to_string()
                .replace('\n', "\r\n")
                .into()
        }
    }

    fn render_prompt_indicator(&self, edit_mode: PromptEditMode) -> Cow<'_, str> {
        let indicator: &str = match edit_mode {
            PromptEditMode::Default => self.prompt_indicator.as_deref().unwrap_or("> "),
            PromptEditMode::Emacs => self.prompt_indicator.as_deref().unwrap_or("> "),
            PromptEditMode::Vi(vi_mode) => match vi_mode {
                PromptViMode::Normal => self.vi_normal_prompt_indicator.as_deref().unwrap_or("> "),
                PromptViMode::Insert => self.vi_insert_prompt_indicator.as_deref().unwrap_or(": "),
            },
            PromptEditMode::Custom(str) => &self.default_wrapped_custom_string(str),
        };

        indicator.to_string().into()
    }

    fn render_prompt_multiline_indicator(&self) -> Cow<'_, str> {
        let indicator = match &self.multiline_indicator {
            Some(indicator) => indicator.as_str(),
            None => "::: ",
        };

        indicator.to_string().into()
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
        self.render_right_prompt_on_last_line
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_prompt_does_not_embed_osc_markers() {
        let prompt = NushellPrompt::new();
        let rendered = prompt.render_prompt_left().to_string();

        assert!(!rendered.contains("\x1b]133;"));
        assert!(!rendered.contains("\x1b]633;"));
    }
}
