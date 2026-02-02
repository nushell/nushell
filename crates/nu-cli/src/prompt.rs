use crate::prompt_update::SemanticPromptMode;
use nu_protocol::engine::{EngineState, Stack};
#[cfg(windows)]
use nu_utils::enable_vt_processing;
use reedline::{
    DefaultPrompt, Prompt, PromptEditMode, PromptHistorySearch, PromptHistorySearchStatus,
    PromptViMode,
};
use std::borrow::Cow;

/// Nushell prompt definition
#[derive(Clone)]
pub struct NushellPrompt {
    shell_integration_osc133: bool,
    shell_integration_osc633: bool,
    left_prompt_string: Option<String>,
    right_prompt_string: Option<String>,
    default_prompt_indicator: Option<String>,
    default_vi_insert_prompt_indicator: Option<String>,
    default_vi_normal_prompt_indicator: Option<String>,
    default_multiline_indicator: Option<String>,
    render_right_prompt_on_last_line: bool,
    engine_state: EngineState,
    stack: Stack,
}

impl NushellPrompt {
    pub fn new(
        shell_integration_osc133: bool,
        shell_integration_osc633: bool,
        engine_state: EngineState,
        stack: Stack,
    ) -> NushellPrompt {
        NushellPrompt {
            shell_integration_osc133,
            shell_integration_osc633,
            left_prompt_string: None,
            right_prompt_string: None,
            default_prompt_indicator: None,
            default_vi_insert_prompt_indicator: None,
            default_vi_normal_prompt_indicator: None,
            default_multiline_indicator: None,
            render_right_prompt_on_last_line: false,
            engine_state,
            stack,
        }
    }

    pub fn update_prompt_left(&mut self, prompt_string: Option<String>) {
        self.left_prompt_string = prompt_string;
    }

    pub fn update_prompt_right(
        &mut self,
        prompt_string: Option<String>,
        render_right_prompt_on_last_line: bool,
    ) {
        self.right_prompt_string = prompt_string;
        self.render_right_prompt_on_last_line = render_right_prompt_on_last_line;
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
        render_right_prompt_on_last_line: bool,
    ) {
        let (prompt_vi_insert_string, prompt_vi_normal_string) = prompt_vi;

        self.left_prompt_string = left_prompt_string;
        self.right_prompt_string = right_prompt_string;
        self.default_prompt_indicator = prompt_indicator_string;
        self.default_multiline_indicator = prompt_multiline_indicator_string;

        self.default_vi_insert_prompt_indicator = prompt_vi_insert_string;
        self.default_vi_normal_prompt_indicator = prompt_vi_normal_string;

        self.render_right_prompt_on_last_line = render_right_prompt_on_last_line;
    }

    fn default_wrapped_custom_string(&self, str: String) -> String {
        format!("({str})")
    }

    fn shell_integration_mode(&self) -> SemanticPromptMode {
        SemanticPromptMode::from_config(
            self.shell_integration_osc133,
            self.shell_integration_osc633,
            &self.stack,
            &self.engine_state,
        )
    }

    /// Render a prompt string with semantic markers for the given mode
    fn wrap_prompt_string(&self, prompt: String, mode: SemanticPromptMode) -> String {
        if let SemanticPromptMode::None = mode {
            prompt
        } else {
            let (start, end) = mode.primary_markers();
            format!("{start}{prompt}{end}")
        }
    }

    /// Render a multiline indicator with semantic markers
    fn wrap_multiline_indicator(&self, indicator: &str, mode: SemanticPromptMode) -> String {
        if let SemanticPromptMode::None = mode {
            indicator.to_string()
        } else {
            let (start, end) = mode.secondary_markers();
            format!("{start}{indicator}{end}")
        }
    }

    /// Render a prompt indicator with semantic markers
    fn wrap_prompt_indicator(&self, indicator: &str, mode: SemanticPromptMode) -> String {
        if let SemanticPromptMode::None = mode {
            indicator.to_string()
        } else {
            let (_start, end) = mode.primary_markers();
            format!("\x1b]133;P\x1b\\{indicator}{end}")
        }
    }

    /// Render a right prompt string with semantic markers
    fn wrap_right_prompt(&self, prompt: String, mode: SemanticPromptMode) -> String {
        let marker = mode.right_marker();
        if marker.is_empty() {
            prompt
        } else {
            format!("{marker}{prompt}")
        }
    }
}

impl Prompt for NushellPrompt {
    fn render_prompt_left(&self) -> Cow<'_, str> {
        #[cfg(windows)]
        {
            let _ = enable_vt_processing();
        }

        if let Some(prompt_string) = &self.left_prompt_string {
            prompt_string.replace('\n', "\r\n").into()
        } else {
            let default = DefaultPrompt::default();
            let prompt = default
                .render_prompt_left()
                .to_string()
                .replace('\n', "\r\n");

            let mode = self.shell_integration_mode();
            self.wrap_prompt_string(prompt, mode).into()
        }
    }

    fn render_prompt_right(&self) -> Cow<'_, str> {
        if let Some(prompt_string) = &self.right_prompt_string {
            prompt_string.replace('\n', "\r\n").into()
        } else {
            let default = DefaultPrompt::default();
            let prompt = default
                .render_prompt_right()
                .to_string()
                .replace('\n', "\r\n");

            let mode = self.shell_integration_mode();
            self.wrap_right_prompt(prompt, mode).into()
        }
    }

    fn render_prompt_indicator(&self, edit_mode: PromptEditMode) -> Cow<'_, str> {
        let indicator: &str = match edit_mode {
            PromptEditMode::Default => self.default_prompt_indicator.as_deref().unwrap_or("> "),
            PromptEditMode::Emacs => self.default_prompt_indicator.as_deref().unwrap_or("> "),
            PromptEditMode::Vi(vi_mode) => match vi_mode {
                PromptViMode::Normal => self
                    .default_vi_normal_prompt_indicator
                    .as_deref()
                    .unwrap_or("> "),
                PromptViMode::Insert => self
                    .default_vi_insert_prompt_indicator
                    .as_deref()
                    .unwrap_or(": "),
            },
            PromptEditMode::Custom(str) => &self.default_wrapped_custom_string(str),
        };

        let mode = self.shell_integration_mode();
        self.wrap_prompt_indicator(indicator, mode).into()
    }

    fn render_prompt_multiline_indicator(&self) -> Cow<'_, str> {
        let indicator = match &self.default_multiline_indicator {
            Some(indicator) => indicator.as_str(),
            None => "::: ",
        };

        let mode = self.shell_integration_mode();
        self.wrap_multiline_indicator(indicator, mode).into()
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
