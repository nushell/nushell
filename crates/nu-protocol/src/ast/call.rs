use std::collections::HashMap;

use serde::{Deserialize, Serialize};

use super::Expression;
use crate::{DeclId, Span, Spanned};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum Argument {
    Positional(Expression),
    Named((Spanned<String>, Option<Spanned<String>>, Option<Expression>)),
    Unknown(Expression), // unknown argument used in "fall-through" signatures
}

impl Argument {
    /// The span for an argument
    pub fn span(&self) -> Span {
        match self {
            Argument::Positional(e) => e.span,
            Argument::Named((named, short, expr)) => {
                let start = named.span.start;
                let end = if let Some(expr) = expr {
                    expr.span.end
                } else if let Some(short) = short {
                    short.span.end
                } else {
                    named.span.end
                };

                Span::new(start, end)
            }
            Argument::Unknown(e) => e.span,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Call {
    /// identifier of the declaration to call
    pub decl_id: DeclId,
    pub head: Span,
    pub arguments: Vec<Argument>,
    pub redirect_stdout: bool,
    pub redirect_stderr: bool,
    /// this field is used by the parser to pass additional command-specific information
    pub parser_info: HashMap<String, Expression>,
}

impl Call {
    pub fn new(head: Span) -> Call {
        Self {
            decl_id: 0,
            head,
            arguments: vec![],
            redirect_stdout: true,
            redirect_stderr: false,
            parser_info: HashMap::new(),
        }
    }

    /// The span encompassing the arguments
    ///
    /// If there are no arguments the span covers where the first argument would exist
    ///
    /// If there are one or more arguments the span encompasses the start of the first argument to
    /// end of the last argument
    pub fn arguments_span(&self) -> Span {
        let past = self.head.past();

        let start = self
            .arguments
            .first()
            .map(|a| a.span())
            .unwrap_or(past)
            .start;
        let end = self.arguments.last().map(|a| a.span()).unwrap_or(past).end;

        Span::new(start, end)
    }

    pub fn named_iter(
        &self,
    ) -> impl Iterator<Item = &(Spanned<String>, Option<Spanned<String>>, Option<Expression>)> {
        self.arguments.iter().filter_map(|arg| match arg {
            Argument::Named(named) => Some(named),
            Argument::Positional(_) => None,
            Argument::Unknown(_) => None,
        })
    }

    pub fn named_iter_mut(
        &mut self,
    ) -> impl Iterator<Item = &mut (Spanned<String>, Option<Spanned<String>>, Option<Expression>)>
    {
        self.arguments.iter_mut().filter_map(|arg| match arg {
            Argument::Named(named) => Some(named),
            Argument::Positional(_) => None,
            Argument::Unknown(_) => None,
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

    pub fn add_unknown(&mut self, unknown: Expression) {
        self.arguments.push(Argument::Unknown(unknown));
    }

    pub fn positional_iter(&self) -> impl Iterator<Item = &Expression> {
        self.arguments.iter().filter_map(|arg| match arg {
            Argument::Named(_) => None,
            Argument::Positional(positional) => Some(positional),
            Argument::Unknown(unknown) => Some(unknown),
        })
    }

    pub fn positional_iter_mut(&mut self) -> impl Iterator<Item = &mut Expression> {
        self.arguments.iter_mut().filter_map(|arg| match arg {
            Argument::Named(_) => None,
            Argument::Positional(positional) => Some(positional),
            Argument::Unknown(unknown) => Some(unknown),
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

    pub fn get_parser_info(&self, name: &str) -> Option<&Expression> {
        self.parser_info.get(name)
    }

    pub fn set_parser_info(&mut self, name: String, val: Expression) -> Option<Expression> {
        self.parser_info.insert(name, val)
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

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn argument_span_named() {
        let named = Spanned {
            item: "named".to_string(),
            span: Span::new(2, 3),
        };
        let short = Spanned {
            item: "short".to_string(),
            span: Span::new(5, 7),
        };
        let expr = Expression::garbage(Span::new(11, 13));

        let arg = Argument::Named((named.clone(), None, None));

        assert_eq!(Span::new(2, 3), arg.span());

        let arg = Argument::Named((named.clone(), Some(short.clone()), None));

        assert_eq!(Span::new(2, 7), arg.span());

        let arg = Argument::Named((named.clone(), None, Some(expr.clone())));

        assert_eq!(Span::new(2, 13), arg.span());

        let arg = Argument::Named((named.clone(), Some(short.clone()), Some(expr.clone())));

        assert_eq!(Span::new(2, 13), arg.span());
    }

    #[test]
    fn argument_span_positional() {
        let span = Span::new(2, 3);
        let expr = Expression::garbage(span);
        let arg = Argument::Positional(expr);

        assert_eq!(span, arg.span());
    }

    #[test]
    fn argument_span_unknown() {
        let span = Span::new(2, 3);
        let expr = Expression::garbage(span);
        let arg = Argument::Unknown(expr);

        assert_eq!(span, arg.span());
    }

    #[test]
    fn call_arguments_span() {
        let mut call = Call::new(Span::new(0, 1));
        call.add_positional(Expression::garbage(Span::new(2, 3)));
        call.add_positional(Expression::garbage(Span::new(5, 7)));

        assert_eq!(Span::new(2, 7), call.arguments_span());
    }
}
