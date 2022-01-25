use reedline::DefaultPrompt;

use {
    reedline::{
        Prompt, PromptEditMode, PromptHistorySearch, PromptHistorySearchStatus, PromptViMode,
    },
    std::borrow::Cow,
};

/// Nushell prompt definition
#[derive(Clone)]
pub struct NushellPrompt {
    left_prompt_string: Option<String>,
    right_prompt_string: Option<String>,
    default_prompt_indicator: String,
    default_vi_insert_prompt_indicator: String,
    default_vi_visual_prompt_indicator: String,
    default_menu_prompt_indicator: String,
    default_multiline_indicator: String,
    default_history_prompt_indicator: String,
}

impl Default for NushellPrompt {
    fn default() -> Self {
        NushellPrompt::new()
    }
}

impl NushellPrompt {
    pub fn new() -> NushellPrompt {
        NushellPrompt {
            left_prompt_string: None,
            right_prompt_string: None,
            default_prompt_indicator: "〉".to_string(),
            default_vi_insert_prompt_indicator: ": ".to_string(),
            default_vi_visual_prompt_indicator: "v ".to_string(),
            default_menu_prompt_indicator: "| ".to_string(),
            default_multiline_indicator: "::: ".to_string(),
            default_history_prompt_indicator: "? ".to_string(),
        }
    }

    pub fn update_prompt_left(&mut self, prompt_string: Option<String>) {
        self.left_prompt_string = prompt_string;
    }

    pub fn update_prompt_right(&mut self, prompt_string: Option<String>) {
        self.right_prompt_string = prompt_string;
    }

    pub fn update_prompt_indicator(&mut self, prompt_indicator_string: String) {
        self.default_prompt_indicator = prompt_indicator_string;
    }

    pub fn update_prompt_vi_insert(&mut self, prompt_vi_insert_string: String) {
        self.default_vi_insert_prompt_indicator = prompt_vi_insert_string;
    }

    pub fn update_prompt_vi_visual(&mut self, prompt_vi_visual_string: String) {
        self.default_vi_visual_prompt_indicator = prompt_vi_visual_string;
    }

    pub fn update_prompt_multiline(&mut self, prompt_multiline_indicator_string: String) {
        self.default_multiline_indicator = prompt_multiline_indicator_string;
    }

    pub fn update_all_prompt_strings(
        &mut self,
        left_prompt_string: Option<String>,
        right_prompt_string: Option<String>,
        prompt_indicator_string: String,
        prompt_multiline_indicator_string: String,
        prompt_vi: (String, String),
        prompt_menus: (String, String),
    ) {
        let (prompt_vi_insert_string, prompt_vi_visual_string) = prompt_vi;
        let (prompt_indicator_menu, prompt_history_indicator_menu) = prompt_menus;

        self.left_prompt_string = left_prompt_string;
        self.right_prompt_string = right_prompt_string;
        self.default_prompt_indicator = prompt_indicator_string;
        self.default_vi_insert_prompt_indicator = prompt_vi_insert_string;
        self.default_vi_visual_prompt_indicator = prompt_vi_visual_string;
        self.default_multiline_indicator = prompt_multiline_indicator_string;

        self.default_menu_prompt_indicator = prompt_indicator_menu;
        self.default_history_prompt_indicator = prompt_history_indicator_menu;
    }

    fn default_wrapped_custom_string(&self, str: String) -> String {
        format!("({})", str)
    }
}

impl Prompt for NushellPrompt {
    fn render_prompt_left(&self) -> Cow<str> {
        if let Some(prompt_string) = &self.left_prompt_string {
            prompt_string.into()
        } else {
            let default = DefaultPrompt::new();
            default.render_prompt_left().to_string().into()
        }
    }

    fn render_prompt_right(&self) -> Cow<str> {
        if let Some(prompt_string) = &self.right_prompt_string {
            prompt_string.into()
        } else {
            let default = DefaultPrompt::new();
            default.render_prompt_right().to_string().into()
        }
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
            PromptEditMode::Menu => self.default_menu_prompt_indicator.as_str().into(),
            PromptEditMode::HistoryMenu => self.default_history_prompt_indicator.as_str().into(),
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
