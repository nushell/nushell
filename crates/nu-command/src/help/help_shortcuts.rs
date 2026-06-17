use nu_engine::command_prelude::*;

#[derive(Clone)]
pub struct HelpShortcuts;

impl Command for HelpShortcuts {
    fn name(&self) -> &str {
        "help shortcuts"
    }

    fn description(&self) -> &str {
        "Show help on Reedline keyboard shortcuts."
    }

    fn signature(&self) -> Signature {
        Signature::build("help shortcuts")
            .category(Category::Core)
            .input_output_types(vec![(Type::Nothing, Type::table())])
            .allow_variants_without_examples(true)
    }

    fn run(
        &self,
        _engine_state: &EngineState,
        _stack: &mut Stack,
        call: &Call,
        _input: PipelineData,
    ) -> Result<PipelineData, ShellError> {
        let head = call.head;
        let shortcuts = generate_shortcut_info();
        let mut recs = vec![];

        for shortcut in shortcuts {
            recs.push(Value::record(
                record! {
                    "shortcut" => Value::string(shortcut.shortcut, head),
                    "description" => Value::string(shortcut.description, head),
                },
                head,
            ));
        }

        Ok(Value::list(recs, call.head).into_pipeline_data())
    }
}

struct ShortcutInfo {
    shortcut: String,
    description: String,
}

fn generate_shortcut_info() -> Vec<ShortcutInfo> {
    vec![
        ShortcutInfo {
            shortcut: "!!".into(),
            description: "Repeat the last command".into(),
        },
        ShortcutInfo {
            shortcut: "!n".into(),
            description: "Repeat command number n from history".into(),
        },
        ShortcutInfo {
            shortcut: "!-n".into(),
            description: "Repeat command n steps back from history".into(),
        },
        ShortcutInfo {
            shortcut: "^old^new".into(),
            description: "Replace 'old' with 'new' in last command and execute".into(),
        },
        ShortcutInfo {
            shortcut: "Ctrl+A".into(),
            description: "Move cursor to beginning of line".into(),
        },
        ShortcutInfo {
            shortcut: "Ctrl+E".into(),
            description: "Move cursor to end of line".into(),
        },
        ShortcutInfo {
            shortcut: "Ctrl+K".into(),
            description: "Kill (cut) text from cursor to end of line".into(),
        },
        ShortcutInfo {
            shortcut: "Ctrl+U".into(),
            description: "Kill (cut) text from cursor to beginning of line".into(),
        },
        ShortcutInfo {
            shortcut: "Ctrl+W".into(),
            description: "Kill (cut) word before cursor".into(),
        },
        ShortcutInfo {
            shortcut: "Ctrl+Y".into(),
            description: "Yank (paste) last killed text".into(),
        },
        ShortcutInfo {
            shortcut: "Ctrl+R".into(),
            description: "Reverse search through history".into(),
        },
        ShortcutInfo {
            shortcut: "Ctrl+S".into(),
            description: "Forward search through history".into(),
        },
        ShortcutInfo {
            shortcut: "Ctrl+L".into(),
            description: "Clear the screen".into(),
        },
        ShortcutInfo {
            shortcut: "Ctrl+C".into(),
            description: "Cancel current input / exit".into(),
        },
        ShortcutInfo {
            shortcut: "Ctrl+D".into(),
            description: "Exit nushell (if input is empty)".into(),
        },
        ShortcutInfo {
            shortcut: "Ctrl+Z".into(),
            description: "Suspend nushell (Unix only)".into(),
        },
        ShortcutInfo {
            shortcut: "Ctrl+Left".into(),
            description: "Move cursor one word left".into(),
        },
        ShortcutInfo {
            shortcut: "Ctrl+Right".into(),
            description: "Move cursor one word right".into(),
        },
        ShortcutInfo {
            shortcut: "Alt+B".into(),
            description: "Move cursor one word left".into(),
        },
        ShortcutInfo {
            shortcut: "Alt+F".into(),
            description: "Move cursor one word right".into(),
        },
        ShortcutInfo {
            shortcut: "Alt+D".into(),
            description: "Kill (cut) word after cursor".into(),
        },
        ShortcutInfo {
            shortcut: "Alt+Backspace".into(),
            description: "Kill (cut) word before cursor".into(),
        },
        ShortcutInfo {
            shortcut: "Up Arrow".into(),
            description: "Previous command in history".into(),
        },
        ShortcutInfo {
            shortcut: "Down Arrow".into(),
            description: "Next command in history".into(),
        },
        ShortcutInfo {
            shortcut: "Left Arrow".into(),
            description: "Move cursor left".into(),
        },
        ShortcutInfo {
            shortcut: "Right Arrow".into(),
            description: "Move cursor right".into(),
        },
        ShortcutInfo {
            shortcut: "Tab".into(),
            description: "Accept completion suggestion".into(),
        },
        ShortcutInfo {
            shortcut: "Shift+Tab".into(),
            description: "Cycle through completions backwards".into(),
        },
        ShortcutInfo {
            shortcut: "Enter".into(),
            description: "Execute command".into(),
        },
        ShortcutInfo {
            shortcut: "Escape".into(),
            description: "Cancel current input".into(),
        },
    ]
}

#[cfg(test)]
mod test {
    #[test]
    fn test_examples() -> nu_test_support::Result {
        use super::HelpShortcuts;
        nu_test_support::test().examples(HelpShortcuts)
    }
}
