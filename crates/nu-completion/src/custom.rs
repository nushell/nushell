use crate::engine;
use crate::CompletionContext;
use crate::Suggestion;
use engine::LocationType;
use nu_parser::NewlineMode;
use std::process::Command;
use std::str::from_utf8;

pub struct CustomCompleter<'a> {
    pub(crate) line: &'a str,
    pub(crate) pos: usize,
    pub(crate) locations: Vec<engine::CompletionLocation>,
    pub(crate) context: &'a dyn CompletionContext,
}

impl CustomCompleter<'_> {
    fn completer(&self, cmd: &str) -> Option<Command> {
        if let Some(global_cfg) = &self
            .context
            .source()
            .engine_state
            .configs
            .lock()
            .global_config
        {
            if let Some(completion_vars) = global_cfg.var("completion") {
                for (idx, value) in completion_vars.row_entries() {
                    if idx == cmd {
                        let mut args = Vec::new();
                        for v in value.table_entries() {
                            match v.as_string() {
                                Ok(s) => {
                                    args.push(s);
                                }
                                _ => return None,
                            }
                        }
                        if args.len() > 0 {
                            let mut command = Command::new(&args[0]);
                            if args.len() > 1 {
                                command.args(&args[1..]);
                            }
                            return Some(command);
                        }
                    }
                }
            }
        };
        None
    }

    fn words(&self) -> (usize, Vec<&str>) {
        if self.locations.is_empty() {
            return (self.pos, Vec::new());
        } else {
            let cursor_pos = self.pos;
            let mut pos = self.locations[0].span.start();
            let mut command_start = 0;
            for location in &self.locations {
                if location.span.start() <= cursor_pos && location.span.end() >= cursor_pos {
                    pos = location.span.start();
                }
                if location.span.start() <= cursor_pos {
                    match location.item {
                        LocationType::Command => command_start = location.span.start(),
                        _ => {}
                    }
                }
            }

            let mut words = Vec::new();
            for token in nu_parser::lex(self.line, 0, NewlineMode::Normal).0 {
                if token.span.start() < command_start {
                    // ensure part of current command
                    continue;
                }

                if token.span.start() <= cursor_pos && token.span.end() >= cursor_pos {
                    words.push(&self.line[token.span.start()..cursor_pos]);
                    break;
                } else if token.span.end() < cursor_pos {
                    words.push(token.span.slice(self.line));
                }
            }

            for location in &self.locations {
                if location.span.start() <= cursor_pos && location.span.end() >= cursor_pos {
                    let partial = location.span.slice(self.line).to_string();
                    if partial.len() == 0 {
                        words.push("") // current word being completed is empty
                    }
                }
            }

            (pos, words)
        }
    }

    pub fn complete(&self) -> Option<(usize, Vec<Suggestion>)> {
        let (pos, words) = self.words();

        if words.len() < 2 {
            None
        } else {
            if let Some(mut completer) = self.completer(words[0]) {
                // quick fix quoted words by simply removing `'` and `"` (won't work with those actually containing quotes)
                let patched: Vec<String> = words
                    .into_iter()
                    .map(|w| w.replace("'", "").replace('"', ""))
                    .collect();

                let output = completer.args(patched).output();

                let output_str = match output {
                    Ok(o) => from_utf8(&o.stdout).unwrap_or("").to_owned(),
                    _ => "".to_owned(),
                };

                let suggestions: Vec<Suggestion> =
                    serde_json::from_str(&output_str).unwrap_or(Vec::new());
                Some((pos, suggestions))
            } else {
                None
            }
        }
    }
}
