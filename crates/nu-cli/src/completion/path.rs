use rustyline::completion::FilenameCompleter;

use crate::completion::{Context, Suggestion};

use crate::completion::matchers::Matcher;

pub struct Completer {
    inner: FilenameCompleter,
}

impl Completer {
    pub fn new() -> Completer {
        Completer {
            inner: FilenameCompleter::new(),
        }
    }

    pub fn complete(
        &self,
        _ctx: &Context<'_>,
        partial: &str,
        _matcher: &Box<dyn Matcher>,
    ) -> Vec<Suggestion> {
        let expanded = nu_parser::expand_ndots(partial);

        if let Ok((_pos, pairs)) = self.inner.complete_path(&expanded, expanded.len()) {
            pairs
                .into_iter()
                .map(|v| Suggestion {
                    replacement: v.replacement,
                    display: v.display,
                })
                .collect()
        } else {
            Vec::new()
        }
    }
}
