use rustyline::completion::{Candidate, Completer};
use rustyline::line_buffer::LineBuffer;

#[derive(Debug)]
crate struct NuCompleter;

impl Completer for NuCompleter {
    type Candidate = NuPair;

    fn complete(
        &self,
        _line: &str,
        _pos: usize,
        _context: &rustyline::Context,
    ) -> rustyline::Result<(usize, Vec<NuPair>)> {
        Ok((
            0,
            vec![
                NuPair("exit", "exit"),
                NuPair("ls", "ls"),
                NuPair("ps", "ps"),
            ],
        ))
    }

    fn update(&self, line: &mut LineBuffer, start: usize, elected: &str) {
        let end = line.pos();
        line.replace(start..end, elected)
    }
}

crate struct NuPair(&'static str, &'static str);

impl Candidate for NuPair {
    fn display(&self) -> &str {
        self.0
    }
    fn replacement(&self) -> &str {
        self.1
    }
}
