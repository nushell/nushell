use git2::{Repository, RepositoryOpenFlags};
use std::ffi::OsString;

use crate::prelude::*;

pub struct Prompt;

impl Prompt {
    pub fn new() -> Self {
        Prompt {}
    }

    pub fn render(&self, context: &Context) -> String {
        let cwd = context.shell_manager.path();
        format!(
            "{}{}> ",
            cwd,
            match current_branch(&cwd) {
                Some(s) => format!("({})", s),
                None => "".to_string(),
            }
        )
    }
}

pub fn current_branch(cwd: &str) -> Option<String> {
    let v: Vec<OsString> = vec![];
    match Repository::open_ext(cwd, RepositoryOpenFlags::empty(), v) {
        Ok(repo) => Some(repo.head().ok()?.shorthand()?.to_string()),
        _ => None,
    }
}
