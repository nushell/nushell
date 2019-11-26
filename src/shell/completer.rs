use crate::context::CommandRegistry;

use derive_new::new;
use rustyline::completion::{Completer, FilenameCompleter};

#[derive(new)]
pub(crate) struct NuCompleter {
    pub file_completer: FilenameCompleter,
    pub commands: CommandRegistry,
}

impl NuCompleter {
    pub fn complete(
        &self,
        line: &str,
        pos: usize,
        context: &rustyline::Context,
    ) -> rustyline::Result<(usize, Vec<rustyline::completion::Pair>)> {
        let commands: Vec<String> = self.commands.names();

        let mut completions = self.file_completer.complete(line, pos, context)?.1;

        for completion in &mut completions {
            if completion.replacement.contains("\\ ") {
                completion.replacement = completion.replacement.replace("\\ ", " ");
            }
            if completion.replacement.contains("\\(") {
                completion.replacement = completion.replacement.replace("\\(", "(");
            }

            if completion.replacement.contains(" ") || completion.replacement.contains("(") {
                if !completion.replacement.starts_with("\"") {
                    completion.replacement = format!("\"{}", completion.replacement);
                }
                if !completion.replacement.ends_with("\"") {
                    completion.replacement = format!("{}\"", completion.replacement);
                }
            }
        }

        let line_chars: Vec<_> = line[..pos].chars().collect();
        let mut replace_pos = line_chars.len();
        while replace_pos > 0 {
            if line_chars[replace_pos - 1] == ' ' {
                break;
            }
            replace_pos -= 1;
        }

        for command in commands.iter() {
            let mut pos = replace_pos;
            let mut matched = true;
            if pos < line_chars.len() {
                for chr in command.chars() {
                    if line_chars[pos] != chr {
                        matched = false;
                        break;
                    }
                    pos += 1;
                    if pos == line_chars.len() {
                        break;
                    }
                }
            }

            if matched {
                completions.push(rustyline::completion::Pair {
                    display: command.clone(),
                    replacement: command.clone(),
                });
            }
        }

        Ok((replace_pos, completions))
    }
}
