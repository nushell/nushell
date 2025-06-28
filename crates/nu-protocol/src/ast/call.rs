use std::collections::HashMap;

use serde::{Deserialize, Serialize};

use crate::{
    DeclId, FromValue, ShellError, Span, Spanned, Value, ast::Expression, engine::StateWorkingSet,
    eval_const::eval_constant,
};

/// Parsed command arguments
///
/// Primarily for internal commands
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum Argument {
    /// A positional argument (that is not [`Argument::Spread`])
    ///
    /// ```nushell
    /// my_cmd positional
    /// ```
    Positional(Expression),
    /// A named/flag argument that can optionally receive a [`Value`] as an [`Expression`]
    ///
    /// The optional second `Spanned<String>` refers to the short-flag version if used
    /// ```nushell
    /// my_cmd --flag
    /// my_cmd -f
    /// my_cmd --flag-with-value <expr>
    /// ```
    Named((Spanned<String>, Option<Spanned<String>>, Option<Expression>)),
    /// unknown argument used in "fall-through" signatures
    Unknown(Expression),
    /// a list spread to fill in rest arguments
    Spread(Expression),
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
            Argument::Spread(e) => e.span,
        }
    }

    pub fn expr(&self) -> Option<&Expression> {
        match self {
            Argument::Named((_, _, expr)) => expr.as_ref(),
            Argument::Positional(expr) | Argument::Unknown(expr) | Argument::Spread(expr) => {
                Some(expr)
            }
        }
    }
}

/// Argument passed to an external command
///
/// Here the parsing rules slightly differ to directly pass strings to the external process
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum ExternalArgument {
    /// Expression that needs to be evaluated to turn into an external process argument
    Regular(Expression),
    /// Occurrence of a `...` spread operator that needs to be expanded
    Spread(Expression),
}

impl ExternalArgument {
    pub fn expr(&self) -> &Expression {
        match self {
            ExternalArgument::Regular(expr) => expr,
            ExternalArgument::Spread(expr) => expr,
        }
    }
}

/// Parsed call of a `Command`
///
/// As we also implement some internal keywords in terms of the `Command` trait, this type stores the passed arguments as [`Expression`].
/// Some of its methods lazily evaluate those to [`Value`] while others return the underlying
/// [`Expression`].
///
/// For further utilities check the `nu_engine::CallExt` trait that extends [`Call`]
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Call {
    /// identifier of the declaration to call
    pub decl_id: DeclId,
    pub head: Span,
    pub arguments: Vec<Argument>,
    /// this field is used by the parser to pass additional command-specific information
    pub parser_info: HashMap<String, Expression>,
}

impl Call {
    pub fn new(head: Span) -> Call {
        Self {
            decl_id: DeclId::new(0),
            head,
            arguments: vec![],
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
        if self.arguments.is_empty() {
            self.head.past()
        } else {
            Span::merge_many(self.arguments.iter().map(|a| a.span()))
        }
    }

    pub fn named_iter(
        &self,
    ) -> impl DoubleEndedIterator<Item = &(Spanned<String>, Option<Spanned<String>>, Option<Expression>)>
    {
        self.arguments.iter().filter_map(|arg| match arg {
            Argument::Named(named) => Some(named),
            Argument::Positional(_) => None,
            Argument::Unknown(_) => None,
            Argument::Spread(_) => None,
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
            Argument::Spread(_) => None,
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

    pub fn add_spread(&mut self, args: Expression) {
        self.arguments.push(Argument::Spread(args));
    }

    pub fn positional_iter(&self) -> impl Iterator<Item = &Expression> {
        self.arguments
            .iter()
            .take_while(|arg| match arg {
                Argument::Spread(_) => false, // Don't include positional arguments given to rest parameter
                _ => true,
            })
            .filter_map(|arg| match arg {
                Argument::Named(_) => None,
                Argument::Positional(positional) => Some(positional),
                Argument::Unknown(unknown) => Some(unknown),
                Argument::Spread(_) => None,
            })
    }

    pub fn positional_nth(&self, i: usize) -> Option<&Expression> {
        self.positional_iter().nth(i)
    }

    pub fn positional_len(&self) -> usize {
        self.positional_iter().count()
    }

    /// Returns every argument to the rest parameter, as well as whether each argument
    /// is spread or a normal positional argument (true for spread, false for normal)
    pub fn rest_iter(&self, start: usize) -> impl Iterator<Item = (&Expression, bool)> {
        // todo maybe rewrite to be more elegant or something
        let args = self
            .arguments
            .iter()
            .filter_map(|arg| match arg {
                Argument::Named(_) => None,
                Argument::Positional(positional) => Some((positional, false)),
                Argument::Unknown(unknown) => Some((unknown, false)),
                Argument::Spread(args) => Some((args, true)),
            })
            .collect::<Vec<_>>();
        let spread_start = args.iter().position(|(_, spread)| *spread).unwrap_or(start);
        args.into_iter().skip(start.min(spread_start))
    }

    pub fn get_parser_info(&self, name: &str) -> Option<&Expression> {
        self.parser_info.get(name)
    }

    pub fn set_parser_info(&mut self, name: String, val: Expression) -> Option<Expression> {
        self.parser_info.insert(name, val)
    }

    pub fn get_flag_expr(&self, flag_name: &str) -> Option<&Expression> {
        for name in self.named_iter().rev() {
            if flag_name == name.0.item {
                return name.2.as_ref();
            }
        }

        None
    }

    pub fn get_named_arg(&self, flag_name: &str) -> Option<Spanned<String>> {
        for name in self.named_iter().rev() {
            if flag_name == name.0.item {
                return Some(name.0.clone());
            }
        }

        None
    }

    /// Check if a boolean flag is set (i.e. `--bool` or `--bool=true`)
    /// evaluating the expression after = as a constant command
    pub fn has_flag_const(
        &self,
        working_set: &StateWorkingSet,
        flag_name: &str,
    ) -> Result<bool, ShellError> {
        for name in self.named_iter() {
            if flag_name == name.0.item {
                return if let Some(expr) = &name.2 {
                    // Check --flag=false
                    let result = eval_constant(working_set, expr)?;
                    match result {
                        Value::Bool { val, .. } => Ok(val),
                        _ => Err(ShellError::CantConvert {
                            to_type: "bool".into(),
                            from_type: result.get_type().to_string(),
                            span: result.span(),
                            help: Some("".into()),
                        }),
                    }
                } else {
                    Ok(true)
                };
            }
        }

        Ok(false)
    }

    pub fn get_flag_const<T: FromValue>(
        &self,
        working_set: &StateWorkingSet,
        name: &str,
    ) -> Result<Option<T>, ShellError> {
        if let Some(expr) = self.get_flag_expr(name) {
            let result = eval_constant(working_set, expr)?;
            FromValue::from_value(result).map(Some)
        } else {
            Ok(None)
        }
    }

    pub fn rest_const<T: FromValue>(
        &self,
        working_set: &StateWorkingSet,
        starting_pos: usize,
    ) -> Result<Vec<T>, ShellError> {
        let mut output = vec![];

        for result in
            self.rest_iter_flattened(starting_pos, |expr| eval_constant(working_set, expr))?
        {
            output.push(FromValue::from_value(result)?);
        }

        Ok(output)
    }

    pub fn rest_iter_flattened<F>(
        &self,
        start: usize,
        mut eval: F,
    ) -> Result<Vec<Value>, ShellError>
    where
        F: FnMut(&Expression) -> Result<Value, ShellError>,
    {
        let mut output = Vec::new();

        for (expr, spread) in self.rest_iter(start) {
            let result = eval(expr)?;
            if spread {
                match result {
                    Value::List { mut vals, .. } => output.append(&mut vals),
                    _ => return Err(ShellError::CannotSpreadAsList { span: expr.span }),
                }
            } else {
                output.push(result);
            }
        }

        Ok(output)
    }

    pub fn req_const<T: FromValue>(
        &self,
        working_set: &StateWorkingSet,
        pos: usize,
    ) -> Result<T, ShellError> {
        if let Some(expr) = self.positional_nth(pos) {
            let result = eval_constant(working_set, expr)?;
            FromValue::from_value(result)
        } else if self.positional_len() == 0 {
            Err(ShellError::AccessEmptyContent { span: self.head })
        } else {
            Err(ShellError::AccessBeyondEnd {
                max_idx: self.positional_len() - 1,
                span: self.head,
            })
        }
    }

    pub fn span(&self) -> Span {
        self.head.merge(self.arguments_span())
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use crate::engine::EngineState;

    #[test]
    fn argument_span_named() {
        let engine_state = EngineState::new();
        let mut working_set = StateWorkingSet::new(&engine_state);

        let named = Spanned {
            item: "named".to_string(),
            span: Span::new(2, 3),
        };
        let short = Spanned {
            item: "short".to_string(),
            span: Span::new(5, 7),
        };
        let expr = Expression::garbage(&mut working_set, Span::new(11, 13));

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
        let engine_state = EngineState::new();
        let mut working_set = StateWorkingSet::new(&engine_state);

        let span = Span::new(2, 3);
        let expr = Expression::garbage(&mut working_set, span);
        let arg = Argument::Positional(expr);

        assert_eq!(span, arg.span());
    }

    #[test]
    fn argument_span_unknown() {
        let engine_state = EngineState::new();
        let mut working_set = StateWorkingSet::new(&engine_state);

        let span = Span::new(2, 3);
        let expr = Expression::garbage(&mut working_set, span);
        let arg = Argument::Unknown(expr);

        assert_eq!(span, arg.span());
    }

    #[test]
    fn call_arguments_span() {
        let engine_state = EngineState::new();
        let mut working_set = StateWorkingSet::new(&engine_state);

        let mut call = Call::new(Span::new(0, 1));
        call.add_positional(Expression::garbage(&mut working_set, Span::new(2, 3)));
        call.add_positional(Expression::garbage(&mut working_set, Span::new(5, 7)));

        assert_eq!(Span::new(2, 7), call.arguments_span());
    }
}
