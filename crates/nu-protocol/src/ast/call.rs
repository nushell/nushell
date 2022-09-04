use serde::{Deserialize, Serialize};

use super::Expression;
use crate::{DeclId, Span, Spanned};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum Argument {
    Positional(Expression),
    Named((Spanned<String>, Option<Spanned<String>>, Option<Expression>)),
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Call {
    /// identifier of the declaration to call
    pub decl_id: DeclId,
    pub head: Span,
    pub arguments: Vec<Argument>,
    pub redirect_stdout: bool,
    pub redirect_stderr: bool,
}

impl Call {
    pub fn new(head: Span) -> Call {
        Self {
            decl_id: 0,
            head,
            arguments: vec![],
            redirect_stdout: true,
            redirect_stderr: false,
        }
    }

    pub fn named_iter(
        &self,
    ) -> impl Iterator<Item = &(Spanned<String>, Option<Spanned<String>>, Option<Expression>)> {
        self.arguments.iter().filter_map(|arg| match arg {
            Argument::Named(named) => Some(named),
            Argument::Positional(_) => None,
        })
    }

    pub fn named_iter_mut(
        &mut self,
    ) -> impl Iterator<Item = &mut (Spanned<String>, Option<Spanned<String>>, Option<Expression>)>
    {
        self.arguments.iter_mut().filter_map(|arg| match arg {
            Argument::Named(named) => Some(named),
            Argument::Positional(_) => None,
        })
    }

    pub fn named_len(&self) -> usize {
        self.named_iter().count()
    }

    pub fn add_named(
        &mut self,
        named: (Spanned<String>, Option<Spanned<String>>, Option<Expression>),
    ) {
        self.arguments.push(Argument::Named(named));
    }

    pub fn add_positional(&mut self, positional: Expression) {
        self.arguments.push(Argument::Positional(positional));
    }

    pub fn positional_iter(&self) -> impl Iterator<Item = &Expression> {
        self.arguments.iter().filter_map(|arg| match arg {
            Argument::Named(_) => None,
            Argument::Positional(positional) => Some(positional),
        })
    }

    pub fn positional_iter_mut(&mut self) -> impl Iterator<Item = &mut Expression> {
        self.arguments.iter_mut().filter_map(|arg| match arg {
            Argument::Named(_) => None,
            Argument::Positional(positional) => Some(positional),
        })
    }

    pub fn positional_nth(&self, i: usize) -> Option<&Expression> {
        self.positional_iter().nth(i)
    }

    pub fn positional_nth_mut(&mut self, i: usize) -> Option<&mut Expression> {
        self.positional_iter_mut().nth(i)
    }

    pub fn positional_len(&self) -> usize {
        self.positional_iter().count()
    }

    pub fn has_flag(&self, flag_name: &str) -> bool {
        for name in self.named_iter() {
            if flag_name == name.0.item {
                return true;
            }
        }

        false
    }

    pub fn get_flag_expr(&self, flag_name: &str) -> Option<Expression> {
        for name in self.named_iter() {
            if flag_name == name.0.item {
                return name.2.clone();
            }
        }

        None
    }

    pub fn get_named_arg(&self, flag_name: &str) -> Option<Spanned<String>> {
        for name in self.named_iter() {
            if flag_name == name.0.item {
                return Some(name.0.clone());
            }
        }

        None
    }

    pub fn span(&self) -> Span {
        let mut span = self.head;

        for positional in self.positional_iter() {
            if positional.span.end > span.end {
                span.end = positional.span.end;
            }
        }

        for (named, _, val) in self.named_iter() {
            if named.span.end > span.end {
                span.end = named.span.end;
            }

            if let Some(val) = &val {
                if val.span.end > span.end {
                    span.end = val.span.end;
                }
            }
        }

        span
    }
}
