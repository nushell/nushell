use crossterm::{Color, Colored};
use git2::{Repository, RepositoryOpenFlags};
use std::ffi::OsString;
use std::fmt;
use std::str::FromStr;

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
                Node::Color(color) => {
                    let _ = color.fmt(f);
                }
            }
        }
        Ok(())
    }
}

fn parse(spec: &str) -> Result<Vec<Node>, String> {
    let (mut out, mut cur) = (vec![], String::new());

    let (mut state, mut escape) = (State::Literal, false);
    for c in spec.chars() {
        state = match c {
            c if escape => {
                cur.push(c);
                escape = false;
                state
            }
            '{' => {
                if !cur.is_empty() {
                    out.push(Node::Literal(cur.clone()));
                    cur.clear();
                }
                State::Value
            }
            '}' => {
                if state != State::Value {
                    return Err("value end without starting".to_string());
                }

                out.push(Node::Value(cur.parse()?));
                cur.clear();
                State::Literal
            }
            '<' => {
                if !cur.is_empty() {
                    out.push(Node::Literal(cur.clone()));
                    cur.clear();
                }
                State::Style
            }
            '>' => {
                if state != State::Style {
                    return Err("style end without starting".to_string());
                }

                let mut iter = str::splitn(&cur, 2, ':');
                let prefix = match iter.next() {
                    Some(p) => p,
                    None => return Err("invalid: empty style string".to_string()),
                };

                let (foreground, name) = if prefix == "b" {
                    (false, iter.next())
                } else if prefix == "f" {
                    (true, iter.next())
                } else {
                    // foreground is the default
                    (true, Some(prefix))
                };

                let color: Color = match name {
                    Some(n) => n.parse().map_err(|_| "invalid color name".to_string())?,
                    _ => return Err("invalid: empty style string".to_string()),
                };

                if foreground {
                    out.push(Node::Color(Colored::Fg(color)));
                } else {
                    out.push(Node::Color(Colored::Bg(color)));
                }

                cur.clear();
                State::Literal
            }
            '\\' if escape => {
                cur.push('\\');
                state
            }
            '\\' => {
                escape = true;
                state
            }
            c => {
                cur.push(c);
                state
            }
        }
    }

    if state != State::Literal {
        return Err("invalid: must end in literal state".to_string());
    } else if !cur.is_empty() {
        out.push(Node::Literal(cur));
    }

    Ok(out)
}

#[derive(Debug, PartialEq)]
enum Node {
    Literal(String),
    Value(Value),
    Color(Colored),
}

#[derive(Debug, PartialEq)]
enum Value {
    Cwd,
    VcsBranch,
}

impl FromStr for Value {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Ok(match s {
            "cwd" => Value::Cwd,
            "vcs_branch" => Value::VcsBranch,
            _ => return Err(format!("invalid value '{}'", s)),
        })
    }
}

#[derive(PartialEq)]
enum State {
    Literal,
    Value,
    Style,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn prompt_simple() {
        assert_eq!(
            parse("{cwd}{vcs_branch}\\> ").unwrap(),
            vec![
                Node::Value(Value::Cwd),
                Node::Value(Value::VcsBranch),
                Node::Literal("> ".to_string()),
            ]
        )
    }

    #[test]
    fn prompt_colors() {
        assert_eq!(
            parse("<blue>{cwd}<green>{vcs_branch}<reset> $ ").unwrap(),
            vec![
                Node::Color(Colored::Fg(Color::Blue)),
                Node::Value(Value::Cwd),
                Node::Color(Colored::Fg(Color::Green)),
                Node::Value(Value::VcsBranch),
                Node::Color(Colored::Fg(Color::White)),
                Node::Literal(" $ ".to_string()),
            ]
        )
    }

    #[test]
    fn prompt_unclosed_value() {
        assert!(parse("{ ").is_err())
    }

    #[test]
    fn prompt_unclosed_style() {
        assert!(parse("< ").is_err())
    }
}
