use git2::{Repository, RepositoryOpenFlags};
use std::ffi::OsString;

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

pub fn current_branch() -> Option<String> {
    let v: Vec<OsString> = vec![];
    match Repository::open_ext(".", RepositoryOpenFlags::empty(), v) {
        Ok(repo) => {
            let r = repo.head();
            match r {
                Ok(r) => match r.shorthand() {
                    Some(s) => Some(s.to_string()),
                    None => None,
                },
                _ => None,
            }
        }
        _ => None,
    }
}
