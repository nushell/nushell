use git2::{Repository, RepositoryOpenFlags};
use std::ffi::OsString;
use std::fmt;

use crate::prelude::*;

pub struct Prompt {
    ast: Vec<Node>,
}

impl Prompt {
    pub fn new() -> Self {
        Prompt {
            ast: vec![
                Node::Value(Value::Cwd),
                Node::Value(Value::VcsBranch),
                Node::Literal("> ".to_string()),
            ],
        }
    }

    pub fn render(&self, context: &Context) -> String {
        format!(
            "{}",
            Render {
                ast: &self.ast,
                context,
            }
        )
    }
}

struct Render<'a> {
    ast: &'a [Node],
    context: &'a Context,
}

impl<'a> Render<'a> {
    fn cwd(&self, f: &mut fmt::Formatter<'_>) {
        let _ = f.write_str(&self.context.shell_manager.path());
    }

    fn vcs_branch(&self, f: &mut fmt::Formatter<'_>) {
        let cwd = self.context.shell_manager.path();

        let v: Vec<OsString> = vec![];
        let name = Repository::open_ext(cwd, RepositoryOpenFlags::empty(), v)
            .ok()
            .and_then(|repo| Some(repo.head().ok()?.shorthand()?.to_string()));

        if let Some(name) = name {
            let _ = f.write_str(&name);
        }
    }
}

impl fmt::Display for Render<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        for node in self.ast.iter() {
            match node {
                Node::Literal(s) => {
                    f.write_str(s)?;
                }
                Node::Value(Value::Cwd) => {
                    self.cwd(f);
                }
                Node::Value(Value::VcsBranch) => {
                    self.vcs_branch(f);
                }
            }
        }
        Ok(())
    }
}

enum Node {
    Literal(String),
    Value(Value),
}

enum Value {
    Cwd,
    VcsBranch,
}
