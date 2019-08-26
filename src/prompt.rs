use crate::git::current_branch;
use crate::prelude::*;

pub struct Prompt;

impl Prompt {
    pub fn new() -> Self {
        Prompt {}
    }

    pub fn render(&self, context: &Context) -> String {
        format!(
            "{}{}> ",
            context.shell_manager.path(),
            match current_branch() {
                Some(s) => format!("({})", s),
                None => "".to_string(),
            }
        )
    }
}
