use git2::Repository;

pub fn current_branch() -> Option<String> {
    match Repository::open(".") {
        Ok(repo) => {
            let r = repo.head();
            match r {
                Ok(r) => {
                    match r.shorthand() {
                        Some(s) => Some(s.to_string()),
                        None => None,
                    }
                },
                _ => None
            }
        },
        _ => None
    }
}
