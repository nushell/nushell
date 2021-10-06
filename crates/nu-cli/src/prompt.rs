use {
    reedline::{
        Prompt, PromptEditMode, PromptHistorySearch, PromptHistorySearchStatus, PromptViMode,
    },
    std::borrow::Cow,
};

/// Nushell prompt definition
#[derive(Clone)]
pub struct NushellPrompt {
    prompt_command: String,
    prompt_string: String,
    // These are part of the struct definition in case we want to allow
    // further customization to the shell status
    default_prompt_indicator: String,
    default_vi_insert_prompt_indicator: String,
    default_vi_visual_prompt_indicator: String,
    default_multiline_indicator: String,
}

impl Default for NushellPrompt {
    fn default() -> Self {
        NushellPrompt::new()
    }
}

impl NushellPrompt {
    pub fn new() -> NushellPrompt {
        NushellPrompt {
            prompt_command: "".to_string(),
            prompt_string: "".to_string(),
            default_prompt_indicator: "〉".to_string(),
            default_vi_insert_prompt_indicator: ": ".to_string(),
            default_vi_visual_prompt_indicator: "v ".to_string(),
            default_multiline_indicator: "::: ".to_string(),
        }
    }

    pub fn is_new_prompt(&self, prompt_command: &str) -> bool {
        self.prompt_command != prompt_command
    }

    pub fn update_prompt(&mut self, prompt_command: String, prompt_string: String) {
        self.prompt_command = prompt_command;
        self.prompt_string = prompt_string;
    }

    fn default_wrapped_custom_string(&self, str: String) -> String {
        format!("({})", str)
    }
}

impl Prompt for NushellPrompt {
    fn render_prompt(&self, _: usize) -> Cow<str> {
        self.prompt_string.as_str().into()
    }

    fn render_prompt_indicator(&self, edit_mode: PromptEditMode) -> Cow<str> {
        match edit_mode {
            PromptEditMode::Default => self.default_prompt_indicator.as_str().into(),
            PromptEditMode::Emacs => self.default_prompt_indicator.as_str().into(),
            PromptEditMode::Vi(vi_mode) => match vi_mode {
                PromptViMode::Normal => self.default_prompt_indicator.as_str().into(),
                PromptViMode::Insert => self.default_vi_insert_prompt_indicator.as_str().into(),
                PromptViMode::Visual => self.default_vi_visual_prompt_indicator.as_str().into(),
            },
            PromptEditMode::Custom(str) => self.default_wrapped_custom_string(str).into(),
        }
    }

    fn render_prompt_multiline_indicator(&self) -> Cow<str> {
        Cow::Borrowed(self.default_multiline_indicator.as_str())
    }

    fn render_prompt_history_search_indicator(
        &self,
        history_search: PromptHistorySearch,
    ) -> Cow<str> {
        let prefix = match history_search.status {
            PromptHistorySearchStatus::Passing => "",
            PromptHistorySearchStatus::Failing => "failing ",
        };

        Cow::Owned(format!(
            "({}reverse-search: {})",
            prefix, history_search.term
        ))
    }
}
