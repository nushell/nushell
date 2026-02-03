use crate::prompt_update::SemanticPromptMode;
use log::error;
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
    pub shell_integration_osc133: bool,
    pub shell_integration_osc633: bool,
    pub left_prompt: Option<String>,
    pub right_prompt: Option<String>,
    pub prompt_indicator: Option<String>,
    pub vi_insert_prompt_indicator: Option<String>,
    pub vi_normal_prompt_indicator: Option<String>,
    pub multiline_indicator: Option<String>,
    pub render_right_prompt_on_last_line: bool,
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
            left_prompt: None,
            right_prompt: None,
            prompt_indicator: None,
            vi_insert_prompt_indicator: None,
            vi_normal_prompt_indicator: None,
            multiline_indicator: None,
            render_right_prompt_on_last_line: false,
            engine_state,
            stack,
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

    pub(crate) fn shell_integration_mode(&self) -> SemanticPromptMode {
        SemanticPromptMode::from_config(
            self.shell_integration_osc133,
            self.shell_integration_osc633,
            &self.stack,
            &self.engine_state,
        )
    }

    /// Render a prompt string with semantic markers for the given mode
    pub(crate) fn wrap_prompt_string(&self, prompt: String, mode: SemanticPromptMode) -> String {
        if let SemanticPromptMode::None = mode {
            prompt
        } else {
            let (start, end) = mode.primary_markers();
            format!("{start}{prompt}{end}")
        }
    }

    /// Render a multiline indicator with semantic markers
    pub(crate) fn wrap_multiline_indicator(
        &self,
        indicator: &str,
        _mode: SemanticPromptMode,
    ) -> String {
        //TODO: doesn't seem to need wrapping
        //if let SemanticPromptMode::None = mode {
        indicator.to_string()
        // } else {
        //     let (start, end) = mode.secondary_markers();
        //     format!("{start}{indicator}{end}")
        // }
    }

    /// Render a prompt indicator with semantic markers
    pub(crate) fn wrap_prompt_indicator(
        &self,
        indicator: &str,
        mode: SemanticPromptMode,
    ) -> String {
        if let SemanticPromptMode::None = mode {
            indicator.to_string()
        } else {
            // The prompt indicator is always at the start of the prompt
            let (start, end) = mode.start_left_indicator_markers();
            format!("{start}{indicator}{end}")
        }
    }

    /// Render a right prompt string with semantic markers
    pub(crate) fn wrap_right_prompt(&self, prompt: String, mode: SemanticPromptMode) -> String {
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

        if let Some(prompt_string) = &self.left_prompt {
            let left_prompt: Cow<'_, str> = prompt_string.replace('\n', "\r\n").into();
            error!(
                "Rendered left prompt (provided): {}{}",
                left_prompt.clone().to_string().replace('\x1b', r"\e"),
                "\x1b[1G", // We are in raw mode, so move cursor to start of line
            );
            left_prompt
        } else {
            let default = DefaultPrompt::default();
            let prompt = default
                .render_prompt_left()
                .to_string()
                .replace('\n', "\r\n");

            let mode = self.shell_integration_mode();
            let left_prompt: Cow<'_, str> = self.wrap_prompt_string(prompt, mode).into();
            error!(
                "Rendered left prompt (default): {}{}",
                left_prompt.clone().to_string().replace('\x1b', r"\e"),
                "\x1b[1G", // We are in raw mode, so move cursor to start of line
            );
            left_prompt
        }
    }

    fn render_prompt_right(&self) -> Cow<'_, str> {
        if let Some(prompt_string) = &self.right_prompt {
            let right_prompt: Cow<'_, str> = prompt_string.replace('\n', "\r\n").into();
            error!(
                "Rendered right prompt (provided): {}{}",
                right_prompt.clone().to_string().replace('\x1b', r"\e"),
                "\x1b[1G", // We are in raw mode, so move cursor to start of line
            );
            right_prompt
        } else {
            let default = DefaultPrompt::default();
            let prompt = default
                .render_prompt_right()
                .to_string()
                .replace('\n', "\r\n");

            let mode = self.shell_integration_mode();
            let right_prompt: Cow<'_, str> = self.wrap_right_prompt(prompt, mode).into();
            error!(
                "Rendered right prompt (default): {}{}",
                right_prompt.clone().to_string().replace('\x1b', r"\e"),
                "\x1b[1G", // We are in raw mode, so move cursor to start of line
            );
            right_prompt
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

        let mode = self.shell_integration_mode();
        let prompt_indicator: Cow<'_, str> = self.wrap_prompt_indicator(indicator, mode).into();
        error!(
            "Rendered prompt indicator      : {}{}",
            prompt_indicator.clone().to_string().replace('\x1b', r"\e"),
            "\x1b[1G", // We are in raw mode, so move cursor to start of line
        );
        prompt_indicator
    }

    fn render_prompt_multiline_indicator(&self) -> Cow<'_, str> {
        let indicator = match &self.multiline_indicator {
            Some(indicator) => indicator.as_str(),
            None => "::: ",
        };

        let mode = self.shell_integration_mode();
        let multiline_indicator: Cow<'_, str> =
            self.wrap_multiline_indicator(indicator, mode).into();
        error!(
            "Rendered multiline indicator   : {}{}",
            multiline_indicator
                .clone()
                .to_string()
                .replace('\x1b', r"\e"),
            "\x1b[1G",
        );
        multiline_indicator
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
