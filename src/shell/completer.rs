use rustyline::completion::Completer;
use rustyline::completion::{self, FilenameCompleter};
use rustyline::line_buffer::LineBuffer;

crate struct NuCompleter {
    pub file_completer: FilenameCompleter,
}

impl Completer for NuCompleter {
    type Candidate = completion::Pair;

    fn complete(
        &self,
        line: &str,
        pos: usize,
        context: &rustyline::Context,
    ) -> rustyline::Result<(usize, Vec<completion::Pair>)> {
        let mut pairs = vec![
            completion::Pair {
                display: "exit".to_string(),
                replacement: "exit".to_string(),
            },
            completion::Pair {
                display: "ls".to_string(),
                replacement: "ls".to_string(),
            },
            completion::Pair {
                display: "ps".to_string(),
                replacement: "ps".to_string(),
            },
        ];

        let mut completions = self.file_completer.complete(line, pos, context)?.1;
        completions.append(&mut pairs);

        let line_chars: Vec<_> = line.chars().collect();
        let mut replace_pos = pos;
        while replace_pos > 0 {
            if line_chars[replace_pos - 1] == ' ' {
                break;
            }
            replace_pos -= 1;
        }

        Ok((replace_pos, completions))
    }

    fn update(&self, line: &mut LineBuffer, start: usize, elected: &str) {
        let end = line.pos();
        line.replace(start..end, elected)
    }
}
