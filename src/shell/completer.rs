use crate::prelude::*;
use derive_new::new;
use rustyline::completion::Completer;
use rustyline::completion::{self, FilenameCompleter};
use rustyline::line_buffer::LineBuffer;

#[derive(new)]
crate struct NuCompleter {
    pub file_completer: FilenameCompleter,
    pub commands: indexmap::IndexMap<String, Arc<dyn Command>>,
}

impl Completer for NuCompleter {
    type Candidate = completion::Pair;

    fn complete(
        &self,
        line: &str,
        pos: usize,
        context: &rustyline::Context,
    ) -> rustyline::Result<(usize, Vec<completion::Pair>)> {
        let commands: Vec<String> = self.commands.keys().cloned().collect();

        let mut completions = self.file_completer.complete(line, pos, context)?.1;

        let line_chars: Vec<_> = line.chars().collect();
        let mut replace_pos = pos;
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
                completions.push(completion::Pair {
                    display: command.to_string(),
                    replacement: command.to_string(),
                });
            }
        }

        Ok((replace_pos, completions))
    }

    fn update(&self, line: &mut LineBuffer, start: usize, elected: &str) {
        let end = line.pos();
        line.replace(start..end, elected)
    }
}
