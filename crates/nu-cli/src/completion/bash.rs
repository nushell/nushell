use super::matchers::Matcher;
use crate::completion::{Completer, CompletionContext, Suggestion};
use nu_engine::EvaluationContext;
use std::{
    io::Read,
    process::{Command, Stdio},
};

pub struct BashCompleter;

impl BashCompleter {
    fn complete_bash(&self, command: &str) -> Vec<Suggestion> {
        // println!("complete_bash: {}", command);
        //TODO: escape characters
        let bash_command = format!(
            "source ~/.config/nu/bash_completions.sh; get_bash_completions \"{}\"",
            command
        );
        let child = Command::new("bash")
            .args(&["-c", bash_command.as_str()])
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .spawn()
            .expect("BashCompleter: Failed to spawn child process");
        let mut stdout = child.stdout.expect("Error opening bash stdout");
        let suggestions = &mut String::new();
        let _strlen = stdout.read_to_string(suggestions);
        let suggestions: Vec<Suggestion> = suggestions
            .split('\n')
            .map(|s| s.trim().to_string())
            .map(|s| Suggestion {
                display: s.clone(),
                replacement: s,
            })
            .collect();

        suggestions
    }
}

impl Completer for BashCompleter {
    fn complete(
        &self,
        ctx: &CompletionContext<'_>,
        partial: &str,
        _matcher: &dyn Matcher,
    ) -> Vec<Suggestion> {
        let _context: &EvaluationContext = ctx.as_ref();
        self.complete_bash(partial)
    }
}
