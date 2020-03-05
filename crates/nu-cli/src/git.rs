#![cfg(not(feature = "starship-prompt"))]

use git2::{Repository, RepositoryOpenFlags};
use std::ffi::OsString;

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
