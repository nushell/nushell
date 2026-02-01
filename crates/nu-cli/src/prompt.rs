use crate::prompt_update::{
    POST_PROMPT_MARKER, PRE_PROMPT_MARKER, PROMPT_KIND_INITIAL, PROMPT_KIND_RIGHT,
    PROMPT_KIND_SECONDARY, ShellIntegrationMode, VSCODE_POST_PROMPT_MARKER,
    VSCODE_PRE_PROMPT_MARKER, VSCODE_PROMPT_KIND_INITIAL, VSCODE_PROMPT_KIND_RIGHT,
    VSCODE_PROMPT_KIND_SECONDARY,
};
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

    fn shell_integration_mode(&self) -> ShellIntegrationMode {
        ShellIntegrationMode::from_config(
            self.shell_integration_osc133,
            self.shell_integration_osc633,
            &self.stack,
            &self.engine_state,
        )
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

            match self.shell_integration_mode() {
                ShellIntegrationMode::Osc633 => {
                    format!("{VSCODE_PRE_PROMPT_MARKER}{VSCODE_PROMPT_KIND_INITIAL}{prompt}{VSCODE_POST_PROMPT_MARKER}").into()
                }
                ShellIntegrationMode::Osc133 => {
                    format!("{PRE_PROMPT_MARKER}{PROMPT_KIND_INITIAL}{prompt}{POST_PROMPT_MARKER}")
                        .into()
                }
                ShellIntegrationMode::None => prompt.into(),
            }
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

            match self.shell_integration_mode() {
                ShellIntegrationMode::Osc633 => {
                    format!("{VSCODE_PROMPT_KIND_RIGHT}{prompt}").into()
                }
                ShellIntegrationMode::Osc133 => format!("{PROMPT_KIND_RIGHT}{prompt}").into(),
                ShellIntegrationMode::None => prompt.into(),
            }
        }
    }

    fn render_prompt_indicator(&self, edit_mode: PromptEditMode) -> Cow<'_, str> {
        match edit_mode {
            PromptEditMode::Default => match &self.default_prompt_indicator {
                Some(indicator) => indicator,
                None => "> ",
            }
            .into(),
            PromptEditMode::Emacs => match &self.default_prompt_indicator {
                Some(indicator) => indicator,
                None => "> ",
            }
            .into(),
            PromptEditMode::Vi(vi_mode) => match vi_mode {
                PromptViMode::Normal => match &self.default_vi_normal_prompt_indicator {
                    Some(indicator) => indicator,
                    None => "> ",
                },
                PromptViMode::Insert => match &self.default_vi_insert_prompt_indicator {
                    Some(indicator) => indicator,
                    None => ": ",
                },
            }
            .into(),
            PromptEditMode::Custom(str) => self.default_wrapped_custom_string(str).into(),
        }
    }

    fn render_prompt_multiline_indicator(&self) -> Cow<'_, str> {
        let indicator = match &self.default_multiline_indicator {
            Some(indicator) => indicator.as_str(),
            None => "::: ",
        };

        match self.shell_integration_mode() {
            ShellIntegrationMode::Osc633 => {
                format!("{VSCODE_PROMPT_KIND_SECONDARY}{indicator}").into()
            }
            ShellIntegrationMode::Osc133 => format!("{PROMPT_KIND_SECONDARY}{indicator}").into(),
            ShellIntegrationMode::None => indicator.into(),
        }
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
