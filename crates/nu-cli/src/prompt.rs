use reedline::DefaultPrompt;

use {
    reedline::{
        Prompt, PromptEditMode, PromptHistorySearch, PromptHistorySearchStatus, PromptViMode,
    },
    std::borrow::Cow,
};

const PROMPT_MARKER_BEFORE_PS1: &str = "\x1b]133;A\x1b\\"; // OSC 133;A ST
const PROMPT_MARKER_BEFORE_PS2: &str = "\x1b]133;A;k=s\x1b\\"; // OSC 133;A;k=s ST

/// Nushell prompt definition
#[derive(Clone)]
pub struct NushellPrompt {
    left_prompt_string: Option<String>,
    right_prompt_string: Option<String>,
    default_prompt_indicator: Option<String>,
    default_vi_insert_prompt_indicator: Option<String>,
    default_vi_normal_prompt_indicator: Option<String>,
    default_multiline_indicator: Option<String>,
    shell_integration: bool,
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
            default_prompt_indicator: None,
            default_vi_insert_prompt_indicator: None,
            default_vi_normal_prompt_indicator: None,
            default_multiline_indicator: None,
            shell_integration: false,
        }
    }

    pub fn update_prompt_left(&mut self, prompt_string: Option<String>) {
        self.left_prompt_string = prompt_string;
    }

    pub fn update_prompt_right(&mut self, prompt_string: Option<String>) {
        self.right_prompt_string = prompt_string;
    }

    pub fn update_prompt_indicator(&mut self, prompt_indicator_string: Option<String>) {
        self.default_prompt_indicator = prompt_indicator_string;
    }

    pub fn update_prompt_vi_insert(&mut self, prompt_vi_insert_string: Option<String>) {
        self.default_vi_insert_prompt_indicator = prompt_vi_insert_string;
    }

    pub fn update_prompt_vi_normal(&mut self, prompt_vi_normal_string: Option<String>) {
        self.default_vi_normal_prompt_indicator = prompt_vi_normal_string;
    }

    pub fn update_prompt_multiline(&mut self, prompt_multiline_indicator_string: Option<String>) {
        self.default_multiline_indicator = prompt_multiline_indicator_string;
    }

    pub fn update_all_prompt_strings(
        &mut self,
        left_prompt_string: Option<String>,
        right_prompt_string: Option<String>,
        prompt_indicator_string: Option<String>,
        prompt_multiline_indicator_string: Option<String>,
        prompt_vi: (Option<String>, Option<String>),
    ) {
        let (prompt_vi_insert_string, prompt_vi_normal_string) = prompt_vi;

        self.left_prompt_string = left_prompt_string;
        self.right_prompt_string = right_prompt_string;
        self.default_prompt_indicator = prompt_indicator_string;
        self.default_multiline_indicator = prompt_multiline_indicator_string;

        self.default_vi_insert_prompt_indicator = prompt_vi_insert_string;
        self.default_vi_normal_prompt_indicator = prompt_vi_normal_string;
    }

    fn default_wrapped_custom_string(&self, str: String) -> String {
        format!("({})", str)
    }

    pub(crate) fn enable_shell_integration(&mut self) {
        self.shell_integration = true
    }
}

impl Prompt for NushellPrompt {
    fn render_prompt_left(&self) -> Cow<str> {
        // Just before starting to draw the PS1 prompt send the escape code (see
        // https://sw.kovidgoyal.net/kitty/shell-integration/#notes-for-shell-developers)
        let mut prompt = if self.shell_integration {
            String::from(PROMPT_MARKER_BEFORE_PS1)
        } else {
            String::new()
        };

        prompt.push_str(&match &self.left_prompt_string {
            Some(prompt_string) => prompt_string.replace('\n', "\r\n"),
            None => {
                let default = DefaultPrompt::new();
                default
                    .render_prompt_left()
                    .to_string()
                    .replace('\n', "\r\n")
            }
        });

        prompt.into()
    }

    fn render_prompt_right(&self) -> Cow<str> {
        if let Some(prompt_string) = &self.right_prompt_string {
            prompt_string.replace('\n', "\r\n").into()
        } else {
            let default = DefaultPrompt::new();
            default
                .render_prompt_right()
                .to_string()
                .replace('\n', "\r\n")
                .into()
        }
    }

    fn render_prompt_indicator(&self, edit_mode: PromptEditMode) -> Cow<str> {
        // Just before starting to draw the PS1 prompt send the escape code (see
        // https://sw.kovidgoyal.net/kitty/shell-integration/#notes-for-shell-developers)
        let mut prompt = if self.shell_integration {
            String::from(PROMPT_MARKER_BEFORE_PS2)
        } else {
            String::new()
        };

        match edit_mode {
            PromptEditMode::Default | PromptEditMode::Emacs => {
                prompt.push_str(
                    self.default_prompt_indicator
                        .as_ref()
                        .unwrap_or(&String::from("〉")),
                );
                prompt.into()
            }
            PromptEditMode::Vi(vi_mode) => match vi_mode {
                PromptViMode::Normal => {
                    prompt.push_str(
                        self.default_vi_normal_prompt_indicator
                            .as_ref()
                            .unwrap_or(&String::from(": ")),
                    );
                    prompt.into()
                }
                PromptViMode::Insert => {
                    prompt.push_str(
                        self.default_vi_insert_prompt_indicator
                            .as_ref()
                            .unwrap_or(&String::from("〉")),
                    );
                    prompt.into()
                }
            },
            PromptEditMode::Custom(str) => {
                prompt.push_str(&self.default_wrapped_custom_string(str));
                prompt.into()
            }
        }
    }

    fn render_prompt_multiline_indicator(&self) -> Cow<str> {
        match &self.default_multiline_indicator {
            Some(indicator) => indicator.as_str().into(),
            None => "::: ".into(),
        }
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
