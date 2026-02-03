use crate::NushellPrompt;
use log::{error, trace, warn};
use nu_engine::ClosureEvalOnce;
use nu_protocol::{
    Config, PipelineData, Value,
    engine::{EngineState, Stack},
    report_shell_error,
};
use reedline::Prompt;

// Name of environment variable where the prompt could be stored
pub(crate) const PROMPT_COMMAND: &str = "PROMPT_COMMAND";
pub(crate) const PROMPT_COMMAND_RIGHT: &str = "PROMPT_COMMAND_RIGHT";
pub(crate) const PROMPT_INDICATOR: &str = "PROMPT_INDICATOR";
pub(crate) const PROMPT_INDICATOR_VI_INSERT: &str = "PROMPT_INDICATOR_VI_INSERT";
pub(crate) const PROMPT_INDICATOR_VI_NORMAL: &str = "PROMPT_INDICATOR_VI_NORMAL";
pub(crate) const PROMPT_MULTILINE_INDICATOR: &str = "PROMPT_MULTILINE_INDICATOR";
pub(crate) const TRANSIENT_PROMPT_COMMAND: &str = "TRANSIENT_PROMPT_COMMAND";
pub(crate) const TRANSIENT_PROMPT_COMMAND_RIGHT: &str = "TRANSIENT_PROMPT_COMMAND_RIGHT";
pub(crate) const TRANSIENT_PROMPT_INDICATOR: &str = "TRANSIENT_PROMPT_INDICATOR";
pub(crate) const TRANSIENT_PROMPT_INDICATOR_VI_INSERT: &str =
    "TRANSIENT_PROMPT_INDICATOR_VI_INSERT";
pub(crate) const TRANSIENT_PROMPT_INDICATOR_VI_NORMAL: &str =
    "TRANSIENT_PROMPT_INDICATOR_VI_NORMAL";
pub(crate) const TRANSIENT_PROMPT_MULTILINE_INDICATOR: &str =
    "TRANSIENT_PROMPT_MULTILINE_INDICATOR";

// ────────────────────────────────────────────────────────────────────────────────
// OSC 133 / OSC 633 SEMANTIC PROMPT MARKERS
// ────────────────────────────────────────────────────────────────────────────────
// These escape sequences help terminals understand prompt vs. input vs. command output.
// We combine OSC 133 A (start prompt) with the k= property to reduce bytes sent.
// OSC 133 B (end prompt / start command input) is placed AFTER all prompt text.

pub(crate) const PROMPT_START_MARKER: &str = "\x1b]133;P\x1b\\"; // Start of primary prompt (k=i = interactive)
pub(crate) const PRE_PROMPT_MARKER: &str = "\x1b]133;A;k=i\x1b\\"; // Start of primary prompt (k=i = interactive)
pub(crate) const PRE_PROMPT_LINE_CONTINUATION_MARKER: &str = "\x1b]133;A;k=s\x1b\\"; // Start of continuation/multiline prompt (k=s = secondary)
pub(crate) const PROMPT_KIND_RIGHT: &str = "\x1b]133;P;k=r\x1b\\"; // Right prompt segment marker (k=r = right)
pub(crate) const PRE_INPUT_MARKER: &str = "\x1b]133;B\x1b\\"; // End prompt, begin user input/command
pub(crate) const PRE_EXECUTION_MARKER: &str = "\x1b]133;C\x1b\\"; // Start command execution
pub(crate) const POST_EXECUTION_MARKER_PREFIX: &str = "\x1b]133;D;"; // End command execution prefix
pub(crate) const POST_EXECUTION_MARKER_SUFFIX: &str = "\x1b\\"; // End command execution suffix

// OSC633 is the same as OSC133 but specifically for VSCode
pub(crate) const VSCODE_PROMPT_START_MARKER: &str = "\x1b]633;P\x1b\\"; // Start of primary prompt for VSCode (k=i = interactive)
pub(crate) const VSCODE_PRE_PROMPT_MARKER: &str = "\x1b]633;A;k=i\x1b\\"; // Start of primary prompt for VSCode (k=i = interactive)
pub(crate) const VSCODE_PRE_PROMPT_LINE_CONTINUATION_MARKER: &str = "\x1b]633;A;k=s\x1b\\"; // Start of continuation/multiline prompt for VSCode (k=s = secondary)
pub(crate) const VSCODE_PROMPT_KIND_RIGHT: &str = "\x1b]633;P;k=r\x1b\\"; // Right prompt segment marker for VSCode (k=r = right)
pub(crate) const VSCODE_PRE_INPUT_MARKER: &str = "\x1b]633;B\x1b\\"; // End prompt, begin user input/command for VSCode
pub(crate) const VSCODE_PRE_EXECUTION_MARKER: &str = "\x1b]633;C\x1b\\"; // Start command execution for VSCode
pub(crate) const VSCODE_POST_EXECUTION_MARKER_PREFIX: &str = "\x1b]633;D;"; // End command execution prefix for VSCode
pub(crate) const VSCODE_POST_EXECUTION_MARKER_SUFFIX: &str = "\x1b\\"; // End command execution suffix for VSCode
pub(crate) const VSCODE_COMMANDLINE_MARKER_PREFIX: &str = "\x1b]633;E;"; // Command line property prefix for VSCode
pub(crate) const VSCODE_COMMANDLINE_MARKER_SUFFIX: &str = "\x1b\\"; // Command line property suffix for VSCode
pub(crate) const VSCODE_CWD_PROPERTY_MARKER_PREFIX: &str = "\x1b]633;P;Cwd="; // Current working directory property prefix for VSCode
pub(crate) const VSCODE_CWD_PROPERTY_MARKER_SUFFIX: &str = "\x1b\\"; // Current working directory property suffix for VSCode

// We've found that sometimes terminals can get stuck in application mode after
// receiving escape sequences, so we provide a reset sequence to return to normal mode.
// This sequence switches cursor keys back to normal / standard behavior.
// "\x1b[?1h" turns it back on.
pub(crate) const RESET_APPLICATION_MODE: &str = "\x1b[?1l";

#[derive(Clone, Copy)]
pub(crate) enum SemanticPromptMode {
    Osc633, // VS Code terminal
    Osc133, // General terminals supporting OSC 133 (preferred when not in VS Code)
    None,   // No semantic prompt support / disabled
}

impl SemanticPromptMode {
    /// Determine which semantic prompt protocol to use based on config and environment
    pub(crate) fn from_config(
        osc133: bool,
        osc633: bool,
        stack: &Stack,
        engine_state: &EngineState,
    ) -> Self {
        if osc633 {
            if stack
                .get_env_var(engine_state, "TERM_PROGRAM")
                .and_then(|v| v.as_str().ok())
                == Some("vscode")
            {
                SemanticPromptMode::Osc633
            } else if osc133 {
                SemanticPromptMode::Osc133
            } else {
                SemanticPromptMode::None
            }
        } else if osc133 {
            SemanticPromptMode::Osc133
        } else {
            SemanticPromptMode::None
        }
    }

    /// Get the markers for primary prompt (left prompt)
    pub(crate) fn primary_markers(&self) -> (&str, &str) {
        match self {
            SemanticPromptMode::Osc633 => (VSCODE_PRE_PROMPT_MARKER, VSCODE_PRE_INPUT_MARKER),
            SemanticPromptMode::Osc133 => (PRE_PROMPT_MARKER, PRE_INPUT_MARKER),
            SemanticPromptMode::None => ("", ""),
        }
    }

    /// Get the markers for start of prompt (left prompt indicator)
    pub(crate) fn start_left_indicator_markers(&self) -> (&str, &str) {
        match self {
            SemanticPromptMode::Osc633 => (VSCODE_PROMPT_START_MARKER, VSCODE_PRE_INPUT_MARKER),
            SemanticPromptMode::Osc133 => (PROMPT_START_MARKER, PRE_INPUT_MARKER),
            SemanticPromptMode::None => ("", ""),
        }
    }

    /// Get the markers for secondary prompt (multiline continuation)
    pub(crate) fn secondary_markers(&self) -> (&str, &str) {
        match self {
            SemanticPromptMode::Osc633 => (
                VSCODE_PRE_PROMPT_LINE_CONTINUATION_MARKER,
                VSCODE_PRE_INPUT_MARKER,
            ),
            SemanticPromptMode::Osc133 => (PRE_PROMPT_LINE_CONTINUATION_MARKER, PRE_INPUT_MARKER),
            SemanticPromptMode::None => ("", ""),
        }
    }

    /// Get the right prompt marker
    pub(crate) fn right_marker(&self) -> &str {
        match self {
            SemanticPromptMode::Osc633 => VSCODE_PROMPT_KIND_RIGHT,
            SemanticPromptMode::Osc133 => PROMPT_KIND_RIGHT,
            SemanticPromptMode::None => "",
        }
    }
}

fn get_prompt_string(
    prompt: &str,
    config: &Config,
    engine_state: &EngineState,
    stack: &mut Stack,
) -> Option<String> {
    stack
        .get_env_var(engine_state, prompt)
        .and_then(|v| match v {
            Value::Closure { val, .. } => {
                let result = ClosureEvalOnce::new(engine_state, stack, val.as_ref().clone())
                    .run_with_input(PipelineData::empty());

                trace!(
                    "get_prompt_string (block) {}:{}:{}",
                    file!(),
                    line!(),
                    column!()
                );

                result
                    .map_err(|err| {
                        report_shell_error(None, engine_state, &err);
                    })
                    .ok()
            }
            Value::String { .. } => Some(PipelineData::value(v.clone(), None)),
            _ => None,
        })
        .and_then(|pipeline_data| {
            let output = pipeline_data.collect_string("", config).ok();
            let ansi_output = output.map(|mut x| {
                // Always reset the color at the start of the right prompt
                // to ensure there is no ansi bleed over
                if x.is_empty() && prompt == PROMPT_COMMAND_RIGHT {
                    x.insert_str(0, "\x1b[0m")
                };

                x
            });
            // Let's keep this for debugging purposes with nu --log-level warn
            warn!("{}:{}:{} {:?}", file!(), line!(), column!(), ansi_output);

            ansi_output
        })
}

pub(crate) fn update_prompt(
    config: &Config,
    engine_state: &EngineState,
    stack: &mut Stack,
    nu_prompt: &mut NushellPrompt,
) {
    let configured_left_prompt_string =
        match get_prompt_string(PROMPT_COMMAND, config, engine_state, stack) {
            Some(s) => s,
            None => "".to_string(),
        };

    let mode = SemanticPromptMode::from_config(
        config.shell_integration.osc133,
        config.shell_integration.osc633,
        stack,
        engine_state,
    );

    // Now that we have the prompt string lets ansify it.
    let left_prompt_string = if let SemanticPromptMode::None = mode {
        configured_left_prompt_string.into()
    } else {
        let (start, end) = mode.primary_markers();
        Some(format!("{start}{configured_left_prompt_string}{end}"))
    };

    let right_prompt_string = get_prompt_string(PROMPT_COMMAND_RIGHT, config, engine_state, stack)
        .map(|rps| {
            let marker = mode.right_marker();
            if marker.is_empty() {
                rps
            } else {
                format!("{marker}{rps}")
            }
        });

    let prompt_indicator_string = get_prompt_string(PROMPT_INDICATOR, config, engine_state, stack);

    let prompt_multiline_string =
        get_prompt_string(PROMPT_MULTILINE_INDICATOR, config, engine_state, stack).map(|pms| {
            if let SemanticPromptMode::None = mode {
                pms
            } else {
                let (start, end) = mode.secondary_markers();
                format!("{start}{pms}{end}")
            }
        });

    let prompt_vi_insert_string =
        get_prompt_string(PROMPT_INDICATOR_VI_INSERT, config, engine_state, stack);

    let prompt_vi_normal_string =
        get_prompt_string(PROMPT_INDICATOR_VI_NORMAL, config, engine_state, stack);

    // apply the other indicators
    nu_prompt.update_all_prompt_strings(
        left_prompt_string,
        right_prompt_string,
        prompt_indicator_string,
        prompt_multiline_string,
        (prompt_vi_insert_string, prompt_vi_normal_string),
        config.render_right_prompt_on_last_line,
    );
    trace!("update_prompt {}:{}:{}", file!(), line!(), column!());

    fn rust_escape_to_shell_escape(s: &Option<String>) -> String {
        s.clone().unwrap_or_default().replace('\x1b', r"\e")
    }

    error!(
        "shell_integration_osc133: {}",
        nu_prompt.shell_integration_osc133
    );
    error!(
        "shell_integration_osc633: {}",
        nu_prompt.shell_integration_osc633
    );
    error!(
        "left_prompt_string: {}",
        rust_escape_to_shell_escape(&nu_prompt.left_prompt)
    );
    error!(
        "right_prompt_string: {}",
        rust_escape_to_shell_escape(&nu_prompt.right_prompt)
    );
    error!(
        "default_prompt_indicator: {}",
        rust_escape_to_shell_escape(&nu_prompt.prompt_indicator)
    );
    error!(
        "default_vi_insert_prompt_indicator: {}",
        rust_escape_to_shell_escape(&nu_prompt.vi_insert_prompt_indicator)
    );
    error!(
        "default_vi_normal_prompt_indicator: {}",
        rust_escape_to_shell_escape(&nu_prompt.vi_normal_prompt_indicator)
    );
    error!(
        "default_multiline_indicator: {}",
        rust_escape_to_shell_escape(&nu_prompt.multiline_indicator)
    );
    error!(
        "render_right_prompt_on_last_line: {}",
        nu_prompt.render_right_prompt_on_last_line
    );
}

/// Construct the transient prompt based on the normal nu_prompt
pub(crate) fn make_transient_prompt(
    config: &Config,
    engine_state: &EngineState,
    stack: &mut Stack,
    nu_prompt: &NushellPrompt,
) -> Box<dyn Prompt> {
    let mut nu_prompt = nu_prompt.clone();
    let mode = nu_prompt.shell_integration_mode();

    if let Some(s) = get_prompt_string(TRANSIENT_PROMPT_COMMAND, config, engine_state, stack) {
        let wrapped = nu_prompt.wrap_prompt_string(s, mode);
        nu_prompt.update_prompt_left(Some(wrapped))
    }

    if let Some(s) = get_prompt_string(TRANSIENT_PROMPT_COMMAND_RIGHT, config, engine_state, stack)
    {
        let wrapped = nu_prompt.wrap_right_prompt(s, mode);
        nu_prompt.update_prompt_right(Some(wrapped), config.render_right_prompt_on_last_line)
    }

    if let Some(s) = get_prompt_string(TRANSIENT_PROMPT_INDICATOR, config, engine_state, stack) {
        nu_prompt.update_prompt_indicator(Some(s))
    }
    if let Some(s) = get_prompt_string(
        TRANSIENT_PROMPT_INDICATOR_VI_INSERT,
        config,
        engine_state,
        stack,
    ) {
        nu_prompt.update_prompt_vi_insert(Some(s))
    }
    if let Some(s) = get_prompt_string(
        TRANSIENT_PROMPT_INDICATOR_VI_NORMAL,
        config,
        engine_state,
        stack,
    ) {
        nu_prompt.update_prompt_vi_normal(Some(s))
    }

    if let Some(s) = get_prompt_string(
        TRANSIENT_PROMPT_MULTILINE_INDICATOR,
        config,
        engine_state,
        stack,
    ) {
        let wrapped = nu_prompt.wrap_multiline_indicator(&s, mode);
        nu_prompt.update_prompt_multiline(Some(wrapped))
    }

    Box::new(nu_prompt)
}
