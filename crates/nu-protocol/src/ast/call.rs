use std::collections::HashMap;

use serde::{Deserialize, Serialize};

use crate::{
    ast::Expression, engine::StateWorkingSet, eval_const::eval_constant, DeclId, FromValue,
    GetSpan, ShellError, Span, SpanId, Spanned, Value,
};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum Argument {
    Positional(Expression),
    Named(
        (
            Spanned<String>,
            Option<Spanned<String>>,
            Option<Expression>,
            SpanId,
        ),
    ),
    Unknown(Expression), // unknown argument used in "fall-through" signatures
    Spread(Expression),  // a list spread to fill in rest arguments
}

impl Argument {
    pub fn span_id(&self) -> SpanId {
        match self {
            Argument::Positional(e) => e.span_id,
            Argument::Named((_, _, _, span_id)) => *span_id,
            Argument::Unknown(e) => e.span_id,
            Argument::Spread(e) => e.span_id,
        }
    }

    /// The span for an argument
    pub fn span(&self, state: &impl GetSpan) -> Span {
        match self {
            Argument::Positional(e) => e.span(state),
            Argument::Named((_, _, _, span_id)) => state.get_span(*span_id),
            Argument::Unknown(e) => e.span(state),
            Argument::Spread(e) => e.span(state),
        }
    }

    pub fn expr(&self) -> Option<&Expression> {
        match self {
            Argument::Named((_, _, expr, _)) => expr.as_ref(),
            Argument::Positional(expr) | Argument::Unknown(expr) | Argument::Spread(expr) => {
                Some(expr)
            }
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum ExternalArgument {
    Regular(Expression),
    Spread(Expression),
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Call {
    /// identifier of the declaration to call
    pub decl_id: DeclId,
    pub head: Span,
    pub arguments: Spanned<Vec<Argument>>,
    /// this field is used by the parser to pass additional command-specific information
    pub parser_info: HashMap<String, Expression>,
}

impl Call {
    pub fn new(head: Span, args_span: Span) -> Call {
        Self {
            decl_id: 0,
            head,
            arguments: Spanned {
                item: vec![],
                span: args_span,
            },
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
        self.arguments.span
    }

    pub fn named_iter(
        &self,
    ) -> impl Iterator<
        Item = &(
            Spanned<String>,
            Option<Spanned<String>>,
            Option<Expression>,
            SpanId,
        ),
    > {
        self.arguments.item.iter().filter_map(|arg| match arg {
            Argument::Named(named) => Some(named),
            Argument::Positional(_) => None,
            Argument::Unknown(_) => None,
            Argument::Spread(_) => None,
        })
    }

    pub fn named_iter_mut(
        &mut self,
    ) -> impl Iterator<
        Item = &mut (
            Spanned<String>,
            Option<Spanned<String>>,
            Option<Expression>,
            SpanId,
        ),
    > {
        self.arguments.item.iter_mut().filter_map(|arg| match arg {
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
        named: (
            Spanned<String>,
            Option<Spanned<String>>,
            Option<Expression>,
            SpanId,
        ),
    ) {
        self.arguments.item.push(Argument::Named(named));
    }

    pub fn add_positional(&mut self, positional: Expression) {
        self.arguments.item.push(Argument::Positional(positional));
    }

    pub fn add_unknown(&mut self, unknown: Expression) {
        self.arguments.item.push(Argument::Unknown(unknown));
    }

    pub fn add_spread(&mut self, args: Expression) {
        self.arguments.item.push(Argument::Spread(args));
    }

    pub fn positional_iter(&self) -> impl Iterator<Item = &Expression> {
        self.arguments
            .item
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
            .item
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
        for name in self.named_iter() {
            if flag_name == name.0.item {
                return name.2.as_ref();
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

        for result in self.rest_iter_flattened(&working_set, starting_pos, |expr| {
            eval_constant(working_set, expr)
        })? {
            output.push(FromValue::from_value(result)?);
        }

        Ok(output)
    }

    pub fn rest_iter_flattened<F>(
        &self,
        state: &impl GetSpan,
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
                    _ => {
                        return Err(ShellError::CannotSpreadAsList {
                            span: expr.span(state),
                        })
                    }
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

    pub fn span(&self, state: &impl GetSpan) -> Span {
        let mut span = self.head;

        for positional in self.positional_iter() {
            if positional.span(state).end > span.end {
                span.end = positional.span(state).end;
            }
        }

        for (named, _, val, _) in self.named_iter() {
            if named.span.end > span.end {
                span.end = named.span.end;
            }

            if let Some(val) = &val {
                if val.span(state).end > span.end {
                    span.end = val.span(state).end;
                }
            }
        }

        span
    }
}
