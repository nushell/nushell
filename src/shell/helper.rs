use crate::parser::hir::TokensIterator;
use crate::parser::nom_input;
use crate::parser::parse::token_tree::TokenNode;
use crate::parser::parse::tokens::RawToken;
use crate::parser::{Pipeline, PipelineElement};
use crate::shell::shell_manager::ShellManager;
use crate::Tagged;
use ansi_term::Color;
use rustyline::completion::Completer;
use rustyline::error::ReadlineError;
use rustyline::highlight::Highlighter;
use rustyline::hint::Hinter;
use std::borrow::Cow::{self, Owned};

pub(crate) struct Helper {
    helper: ShellManager,
}

impl Helper {
    pub(crate) fn new(helper: ShellManager) -> Helper {
        Helper { helper }
    }
}

impl Completer for Helper {
    type Candidate = rustyline::completion::Pair;
    fn complete(
        &self,
        line: &str,
        pos: usize,
        ctx: &rustyline::Context<'_>,
    ) -> Result<(usize, Vec<rustyline::completion::Pair>), ReadlineError> {
        self.helper.complete(line, pos, ctx)
    }
}

/*
impl Completer for Helper {
    type Candidate = rustyline::completion::Pair;

    fn complete(
        &self,
        line: &str,
        pos: usize,
        ctx: &rustyline::Context<'_>,
    ) -> Result<(usize, Vec<rustyline::completion::Pair>), ReadlineError> {
        let result = self.helper.complete(line, pos, ctx);

        result.map(|(x, y)| (x, y.iter().map(|z| z.into()).collect()))
    }
}
*/

impl Hinter for Helper {
    fn hint(&self, line: &str, pos: usize, ctx: &rustyline::Context<'_>) -> Option<String> {
        self.helper.hint(line, pos, ctx)
    }
}

impl Highlighter for Helper {
    fn highlight_prompt<'b, 's: 'b, 'p: 'b>(&'s self, prompt: &'p str, _: bool) -> Cow<'b, str> {
        Owned("\x1b[32m".to_owned() + &prompt[0..prompt.len() - 2] + "\x1b[m> ")
    }

    fn highlight_hint<'h>(&self, hint: &'h str) -> Cow<'h, str> {
        Owned("\x1b[1m".to_owned() + hint + "\x1b[m")
    }

    fn highlight<'l>(&self, line: &'l str, _pos: usize) -> Cow<'l, str> {
        let tokens = crate::parser::pipeline(nom_input(line, uuid::Uuid::nil()));

        match tokens {
            Err(_) => Cow::Borrowed(line),
            Ok((_rest, v)) => {
                let mut out = String::new();
                let pipeline = match v.as_pipeline() {
                    Err(_) => return Cow::Borrowed(line),
                    Ok(v) => v,
                };

                let Pipeline { parts } = pipeline;
                let mut iter = parts.into_iter();

                loop {
                    match iter.next() {
                        None => {
                            return Cow::Owned(out);
                        }
                        Some(token) => {
                            let styled = paint_pipeline_element(&token, line);
                            out.push_str(&styled.to_string());
                        }
                    }
                }
            }
        }
    }

    fn highlight_char(&self, _line: &str, _pos: usize) -> bool {
        true
    }
}

fn paint_token_node(token_node: &TokenNode, line: &str) -> String {
    let styled = match token_node {
        TokenNode::Call(..) => Color::Cyan.bold().paint(token_node.tag().slice(line)),
        TokenNode::Nodes(..) => Color::Green.bold().paint(token_node.tag().slice(line)),
        TokenNode::Whitespace(..) => Color::White.normal().paint(token_node.tag().slice(line)),
        TokenNode::Flag(..) => Color::Black.bold().paint(token_node.tag().slice(line)),
        TokenNode::Member(..) => Color::Yellow.bold().paint(token_node.tag().slice(line)),
        TokenNode::Error(..) => Color::Red.bold().paint(token_node.tag().slice(line)),
        TokenNode::Delimited(..) => Color::White.paint(token_node.tag().slice(line)),
        TokenNode::Pipeline(..) => Color::Blue.normal().paint(token_node.tag().slice(line)),
        TokenNode::Token(Tagged {
            item: RawToken::Number(..),
            ..
        }) => Color::Purple.bold().paint(token_node.tag().slice(line)),
        TokenNode::Token(Tagged {
            item: RawToken::Size(..),
            ..
        }) => Color::Purple.bold().paint(token_node.tag().slice(line)),
        TokenNode::Token(Tagged {
            item: RawToken::GlobPattern,
            ..
        }) => Color::Cyan.normal().paint(token_node.tag().slice(line)),
        TokenNode::Token(Tagged {
            item: RawToken::String(..),
            ..
        }) => Color::Green.normal().paint(token_node.tag().slice(line)),
        TokenNode::Token(Tagged {
            item: RawToken::Variable(..),
            ..
        }) => Color::Yellow.bold().paint(token_node.tag().slice(line)),
        TokenNode::Token(Tagged {
            item: RawToken::Bare,
            ..
        }) => Color::Green.normal().paint(token_node.tag().slice(line)),
        TokenNode::Token(Tagged {
            item: RawToken::ExternalCommand(..),
            ..
        }) => Color::Cyan.bold().paint(token_node.tag().slice(line)),
        TokenNode::Token(Tagged {
            item: RawToken::ExternalWord,
            ..
        }) => Color::Black.bold().paint(token_node.tag().slice(line)),
        TokenNode::Token(Tagged {
            item: RawToken::Operator(..),
            ..
        }) => Color::Black.bold().paint(token_node.tag().slice(line)),
    };

    styled.to_string()
}

fn paint_pipeline_element(pipeline_element: &PipelineElement, line: &str) -> String {
    let mut styled = String::new();

    if let Some(_) = pipeline_element.pipe {
        styled.push_str(&Color::Purple.paint("|"));
    }

    let mut tokens =
        TokensIterator::new(&pipeline_element.tokens, pipeline_element.tokens.tag, false);
    let head = tokens.next();

    match head {
        None => return styled,
        Some(head) => {
            styled.push_str(&Color::Cyan.bold().paint(head.tag().slice(line)).to_string())
        }
    }

    for token in tokens {
        styled.push_str(&paint_token_node(token, line));
    }

    styled.to_string()
}

impl rustyline::Helper for Helper {}
