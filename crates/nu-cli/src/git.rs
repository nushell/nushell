use crate::prelude::*;

pub fn current_branch() -> Option<String> {
    if let Ok(config) = crate::data::config::config(Tag::unknown()) {
        let use_starship =
            matches!(config.get("use_starship"), Some(b) if matches!(b.as_bool(), Ok(b) if b));

        if !use_starship {
            #[cfg(feature = "git2")]
            {
                use git2::{Repository, RepositoryOpenFlags};
                use std::ffi::OsString;

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
            #[cfg(not(feature = "git2"))]
            {
                None
            }
        } else {
            None
        }
    } else {
        None
    }
}
