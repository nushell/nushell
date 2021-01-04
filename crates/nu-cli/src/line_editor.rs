use crate::evaluation_context::EvaluationContext;
use std::error::Error;

#[allow(unused_imports)]
use crate::prelude::*;

#[allow(unused_imports)]
use crate::script::LineResult;

#[cfg(feature = "rustyline-support")]
use crate::shell::Helper;

#[cfg(feature = "rustyline-support")]
use rustyline::{
    self,
    config::Configurer,
    config::{ColorMode, CompletionType, Config},
    error::ReadlineError,
    At, Cmd, Editor, KeyPress, Movement, Word,
};

#[cfg(feature = "rustyline-support")]
pub fn convert_rustyline_result_to_string(input: Result<String, ReadlineError>) -> LineResult {
    match input {
        Ok(s) if s == "history -c" || s == "history --clear" => LineResult::ClearHistory,
        Ok(s) => LineResult::Success(s),
        Err(ReadlineError::Interrupted) => LineResult::CtrlC,
        Err(ReadlineError::Eof) => LineResult::CtrlD,
        Err(err) => {
            outln!("Error: {:?}", err);
            LineResult::Break
        }
    }
}

#[cfg(feature = "rustyline-support")]
pub fn default_rustyline_editor_configuration() -> Editor<Helper> {
    #[cfg(windows)]
    const DEFAULT_COMPLETION_MODE: CompletionType = CompletionType::Circular;
    #[cfg(not(windows))]
    const DEFAULT_COMPLETION_MODE: CompletionType = CompletionType::List;

    let config = Config::builder().color_mode(ColorMode::Forced).build();
    let mut rl: Editor<_> = Editor::with_config(config);

    // add key bindings to move over a whole word with Ctrl+ArrowLeft and Ctrl+ArrowRight
    rl.bind_sequence(
        KeyPress::ControlLeft,
        Cmd::Move(Movement::BackwardWord(1, Word::Vi)),
    );
    rl.bind_sequence(
        KeyPress::ControlRight,
        Cmd::Move(Movement::ForwardWord(1, At::AfterEnd, Word::Vi)),
    );

    // workaround for multiline-paste hang in rustyline (see https://github.com/kkawakam/rustyline/issues/202)
    rl.bind_sequence(KeyPress::BracketedPasteStart, rustyline::Cmd::Noop);

    // Let's set the defaults up front and then override them later if the user indicates
    // defaults taken from here https://github.com/kkawakam/rustyline/blob/2fe886c9576c1ea13ca0e5808053ad491a6fe049/src/config.rs#L150-L167
    rl.set_max_history_size(100);
    rl.set_history_ignore_dups(true);
    rl.set_history_ignore_space(false);
    rl.set_completion_type(DEFAULT_COMPLETION_MODE);
    rl.set_completion_prompt_limit(100);
    rl.set_keyseq_timeout(-1);
    rl.set_edit_mode(rustyline::config::EditMode::Emacs);
    rl.set_auto_add_history(false);
    rl.set_bell_style(rustyline::config::BellStyle::default());
    rl.set_color_mode(rustyline::ColorMode::Enabled);
    rl.set_tab_stop(8);

    if let Err(e) = crate::keybinding::load_keybindings(&mut rl) {
        println!("Error loading keybindings: {:?}", e);
    }

    rl
}

#[cfg(feature = "rustyline-support")]
pub fn configure_rustyline_editor(
    rl: &mut Editor<Helper>,
    config: &dyn nu_data::config::Conf,
) -> Result<(), ShellError> {
    #[cfg(windows)]
    const DEFAULT_COMPLETION_MODE: CompletionType = CompletionType::Circular;
    #[cfg(not(windows))]
    const DEFAULT_COMPLETION_MODE: CompletionType = CompletionType::List;

    if let Some(line_editor_vars) = config.var("line_editor") {
        for (idx, value) in line_editor_vars.row_entries() {
            match idx.as_ref() {
                "max_history_size" => {
                    if let Ok(max_history_size) = value.as_u64() {
                        rl.set_max_history_size(max_history_size as usize);
                    }
                }
                "history_duplicates" => {
                    // history_duplicates = match value.as_string() {
                    //     Ok(s) if s.to_lowercase() == "alwaysadd" => {
                    //         rustyline::config::HistoryDuplicates::AlwaysAdd
                    //     }
                    //     Ok(s) if s.to_lowercase() == "ignoreconsecutive" => {
                    //         rustyline::config::HistoryDuplicates::IgnoreConsecutive
                    //     }
                    //     _ => rustyline::config::HistoryDuplicates::AlwaysAdd,
                    // };
                    if let Ok(history_duplicates) = value.as_bool() {
                        rl.set_history_ignore_dups(history_duplicates);
                    }
                }
                "history_ignore_space" => {
                    if let Ok(history_ignore_space) = value.as_bool() {
                        rl.set_history_ignore_space(history_ignore_space);
                    }
                }
                "completion_type" => {
                    let completion_type = match value.as_string() {
                        Ok(s) if s.to_lowercase() == "circular" => {
                            rustyline::config::CompletionType::Circular
                        }
                        Ok(s) if s.to_lowercase() == "list" => {
                            rustyline::config::CompletionType::List
                        }
                        #[cfg(all(unix, feature = "with-fuzzy"))]
                        Ok(s) if s.to_lowercase() == "fuzzy" => {
                            rustyline::config::CompletionType::Fuzzy
                        }
                        _ => DEFAULT_COMPLETION_MODE,
                    };
                    rl.set_completion_type(completion_type);
                }
                "completion_prompt_limit" => {
                    if let Ok(completion_prompt_limit) = value.as_u64() {
                        rl.set_completion_prompt_limit(completion_prompt_limit as usize);
                    }
                }
                "keyseq_timeout_ms" => {
                    if let Ok(keyseq_timeout_ms) = value.as_u64() {
                        rl.set_keyseq_timeout(keyseq_timeout_ms as i32);
                    }
                }
                "edit_mode" => {
                    let edit_mode = match value.as_string() {
                        Ok(s) if s.to_lowercase() == "vi" => rustyline::config::EditMode::Vi,
                        Ok(s) if s.to_lowercase() == "emacs" => rustyline::config::EditMode::Emacs,
                        _ => rustyline::config::EditMode::Emacs,
                    };
                    rl.set_edit_mode(edit_mode);
                    // Note: When edit_mode is Emacs, the keyseq_timeout_ms is set to -1
                    // no matter what you may have configured. This is so that key chords
                    // can be applied without having to do them in a given timeout. So,
                    // it essentially turns off the keyseq timeout.
                }
                "auto_add_history" => {
                    if let Ok(auto_add_history) = value.as_bool() {
                        rl.set_auto_add_history(auto_add_history);
                    }
                }
                "bell_style" => {
                    let bell_style = match value.as_string() {
                        Ok(s) if s.to_lowercase() == "audible" => {
                            rustyline::config::BellStyle::Audible
                        }
                        Ok(s) if s.to_lowercase() == "none" => rustyline::config::BellStyle::None,
                        Ok(s) if s.to_lowercase() == "visible" => {
                            rustyline::config::BellStyle::Visible
                        }
                        _ => rustyline::config::BellStyle::default(),
                    };
                    rl.set_bell_style(bell_style);
                }
                "color_mode" => {
                    let color_mode = match value.as_string() {
                        Ok(s) if s.to_lowercase() == "enabled" => rustyline::ColorMode::Enabled,
                        Ok(s) if s.to_lowercase() == "forced" => rustyline::ColorMode::Forced,
                        Ok(s) if s.to_lowercase() == "disabled" => rustyline::ColorMode::Disabled,
                        _ => rustyline::ColorMode::Enabled,
                    };
                    rl.set_color_mode(color_mode);
                }
                "tab_stop" => {
                    if let Ok(tab_stop) = value.as_u64() {
                        rl.set_tab_stop(tab_stop as usize);
                    }
                }
                _ => (),
            }
        }
    }

    Ok(())
}

#[cfg(feature = "rustyline-support")]
pub fn nu_line_editor_helper(
    context: &mut EvaluationContext,
    config: &dyn nu_data::config::Conf,
) -> crate::shell::Helper {
    let hinter = rustyline_hinter(config);
    crate::shell::Helper::new(context.clone(), hinter)
}

#[cfg(feature = "rustyline-support")]
pub fn rustyline_hinter(
    config: &dyn nu_data::config::Conf,
) -> Option<rustyline::hint::HistoryHinter> {
    if let Some(line_editor_vars) = config.var("line_editor") {
        for (idx, value) in line_editor_vars.row_entries() {
            if idx == "show_hints" && value.expect_string() == "false" {
                return None;
            }
        }
    }

    Some(rustyline::hint::HistoryHinter {})
}

pub fn configure_ctrl_c(_context: &mut EvaluationContext) -> Result<(), Box<dyn Error>> {
    #[cfg(feature = "ctrlc")]
    {
        let cc = _context.ctrl_c.clone();

        ctrlc::set_handler(move || {
            cc.store(true, Ordering::SeqCst);
        })?;

        if _context.ctrl_c.load(Ordering::SeqCst) {
            _context.ctrl_c.store(false, Ordering::SeqCst);
        }
    }

    Ok(())
}
