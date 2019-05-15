use rustyline::{completion, Context};
use std::collections::BTreeMap;

#[allow(unused)]
crate struct Completer {
    commands: BTreeMap<String, Box<dyn crate::CommandBlueprint>>,
}

impl completion::Completer for Completer {
    type Candidate = completion::Pair;

    fn complete(
        &self,
        _line: &str,
        _pos: usize,
        _ctx: &Context<'_>,
    ) -> rustyline::Result<(usize, Vec<completion::Pair>)> {
        let pairs = self
            .commands
            .keys()
            .map(|k| completion::Pair {
                display: k.clone(),
                replacement: k.clone(),
            })
            .collect();
        Ok((0, pairs))
    }

    fn update(&self, line: &mut rustyline::line_buffer::LineBuffer, start: usize, elected: &str) {
        let end = line.pos();
        line.replace(start..end, elected)
    }
}
