use crate::prompt_update::{
    POST_PROMPT_MARKER, PRE_PROMPT_MARKER, VSCODE_POST_PROMPT_MARKER, VSCODE_PRE_PROMPT_MARKER,
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

            if self.shell_integration_osc633 {
                if self
                    .stack
                    .get_env_var(&self.engine_state, "TERM_PROGRAM")
                    .and_then(|v| v.as_str().ok())
                    == Some("vscode")
                {
                    // We're in vscode and we have osc633 enabled
                    format!("{VSCODE_PRE_PROMPT_MARKER}{prompt}{VSCODE_POST_PROMPT_MARKER}").into()
                } else if self.shell_integration_osc133 {
                    // If we're in VSCode but we don't find the env var, but we have osc133 set, then use it
                    format!("{PRE_PROMPT_MARKER}{prompt}{POST_PROMPT_MARKER}").into()
                } else {
                    prompt.into()
                }
            } else if self.shell_integration_osc133 {
                format!("{PRE_PROMPT_MARKER}{prompt}{POST_PROMPT_MARKER}").into()
            } else {
                prompt.into()
            }
        }
    }

    fn render_prompt_right(&self) -> Cow<'_, str> {
        if let Some(prompt_string) = &self.right_prompt_string {
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
        match &self.default_multiline_indicator {
            Some(indicator) => indicator,
            None => "::: ",
        }
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
        self.render_right_prompt_on_last_line
    }
}
