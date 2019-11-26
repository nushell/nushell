#[cfg(not(coloring_in_tokens))]
use crate::hir::syntax_shape::FlatShape;
use crate::{
    hir::syntax_shape::{
        color_syntax, expand_atom, expand_expr, expand_syntax, AtomicToken, ColorSyntax,
        ExpandContext, ExpandExpression, ExpandSyntax, ExpansionRule, MaybeSpaceShape,
        UnspannedAtomicToken,
    },
    hir::Expression,
    TokensIterator,
};
use nu_errors::ParseError;
use nu_source::{b, DebugDocBuilder, HasSpan, PrettyDebug, Span, Spanned, SpannedItem};

#[derive(Debug, Clone)]
pub struct ExternalTokensSyntax {
    pub tokens: Spanned<Vec<Spanned<String>>>,
}

impl HasSpan for ExternalTokensSyntax {
    fn span(&self) -> Span {
        self.tokens.span
    }
}

impl PrettyDebug for ExternalTokensSyntax {
    fn pretty(&self) -> DebugDocBuilder {
        b::intersperse(
            self.tokens
                .iter()
                .map(|token| b::primitive(format!("{:?}", token.item))),
            b::space(),
        )
    }
}

#[derive(Debug, Copy, Clone)]
pub struct ExternalTokensShape;

impl ExpandSyntax for ExternalTokensShape {
    type Output = ExternalTokensSyntax;

    fn name(&self) -> &'static str {
        "external command"
    }

    fn expand_syntax<'a, 'b>(
        &self,
        token_nodes: &'b mut TokensIterator<'a>,
        context: &ExpandContext,
    ) -> Result<Self::Output, ParseError> {
        let mut out: Vec<Spanned<String>> = vec![];

        let start = token_nodes.span_at_cursor();

        loop {
            match expand_syntax(&ExternalExpressionShape, token_nodes, context) {
                Err(_) | Ok(None) => break,
                Ok(Some(span)) => out.push(span.spanned_string(context.source())),
            }
        }

        let end = token_nodes.span_at_cursor();

        Ok(ExternalTokensSyntax {
            tokens: out.spanned(start.until(end)),
        })
    }
}

#[cfg(not(coloring_in_tokens))]
impl ColorSyntax for ExternalTokensShape {
    type Info = ();
    type Input = ();

    fn color_syntax<'a, 'b>(
        &self,
        _input: &(),
        token_nodes: &'b mut TokensIterator<'a>,
        context: &ExpandContext,
        shapes: &mut Vec<Spanned<FlatShape>>,
    ) -> Self::Info {
        loop {
            // Allow a space
            color_syntax(&MaybeSpaceShape, token_nodes, context, shapes);

            // Process an external expression. External expressions are mostly words, with a
            // few exceptions (like $variables and path expansion rules)
            match color_syntax(&ExternalExpression, token_nodes, context, shapes).1 {
                ExternalExpressionResult::Eof => break,
                ExternalExpressionResult::Processed => continue,
            }
        }
    }
}

#[cfg(coloring_in_tokens)]
impl ColorSyntax for ExternalTokensShape {
    type Info = ();
    type Input = ();

    fn name(&self) -> &'static str {
        "ExternalTokensShape"
    }

    fn color_syntax<'a, 'b>(
        &self,
        _input: &(),
        token_nodes: &'b mut TokensIterator<'a>,
        context: &ExpandContext,
    ) -> Self::Info {
        loop {
            // Allow a space
            color_syntax(&MaybeSpaceShape, token_nodes, context);

            // Process an external expression. External expressions are mostly words, with a
            // few exceptions (like $variables and path expansion rules)
            match color_syntax(&ExternalExpression, token_nodes, context).1 {
                ExternalExpressionResult::Eof => break,
                ExternalExpressionResult::Processed => continue,
            }
        }
    }
}

#[derive(Debug, Copy, Clone)]
pub struct ExternalExpressionShape;

impl ExpandSyntax for ExternalExpressionShape {
    type Output = Option<Span>;

    fn name(&self) -> &'static str {
        "external expression"
    }

    fn expand_syntax<'a, 'b>(
        &self,
        token_nodes: &'b mut TokensIterator<'a>,
        context: &ExpandContext,
    ) -> Result<Self::Output, ParseError> {
        expand_syntax(&MaybeSpaceShape, token_nodes, context)?;

        let first = expand_atom(
            token_nodes,
            "external command",
            context,
            ExpansionRule::new().allow_external_command(),
        )?
        .span;

        let mut last = first;

        loop {
            let continuation = expand_expr(&ExternalContinuationShape, token_nodes, context);

            if let Ok(continuation) = continuation {
                last = continuation.span;
            } else {
                break;
            }
        }

        Ok(Some(first.until(last)))
    }
}

#[derive(Debug, Copy, Clone)]
struct ExternalExpression;

impl ExpandSyntax for ExternalExpression {
    type Output = Option<Span>;

    fn name(&self) -> &'static str {
        "external expression"
    }

    fn expand_syntax<'a, 'b>(
        &self,
        token_nodes: &'b mut TokensIterator<'a>,
        context: &ExpandContext,
    ) -> Result<Self::Output, ParseError> {
        expand_syntax(&MaybeSpaceShape, token_nodes, context)?;

        let first = expand_syntax(&ExternalHeadShape, token_nodes, context)?.span;
        let mut last = first;

        loop {
            let continuation = expand_syntax(&ExternalContinuationShape, token_nodes, context);

            if let Ok(continuation) = continuation {
                last = continuation.span;
            } else {
                break;
            }
        }

        Ok(Some(first.until(last)))
    }
}

#[derive(Debug, Copy, Clone)]
struct ExternalHeadShape;

impl ExpandExpression for ExternalHeadShape {
    fn name(&self) -> &'static str {
        "external argument"
    }

    fn expand_expr<'a, 'b>(
        &self,
        token_nodes: &'b mut TokensIterator<'a>,
        context: &ExpandContext,
    ) -> Result<Expression, ParseError> {
        let atom = expand_atom(
            token_nodes,
            "external argument",
            context,
            ExpansionRule::new()
                .allow_external_word()
                .treat_size_as_word(),
        )?;

        let span = atom.span;

        Ok(match &atom.unspanned {
            UnspannedAtomicToken::Eof { .. } => unreachable!("ExpansionRule doesn't allow EOF"),
            UnspannedAtomicToken::Error { .. } => unreachable!("ExpansionRule doesn't allow Error"),
            UnspannedAtomicToken::Size { .. } => unreachable!("ExpansionRule treats size as word"),
            UnspannedAtomicToken::Whitespace { .. } => {
                unreachable!("ExpansionRule doesn't allow Whitespace")
            }
            UnspannedAtomicToken::ShorthandFlag { .. }
            | UnspannedAtomicToken::SquareDelimited { .. } => {
                return Err(ParseError::mismatch(
                    "external command name",
                    "pipeline".spanned(atom.span),
                ))
            }
            UnspannedAtomicToken::ExternalCommand { command } => {
                Expression::external_command(*command, span)
            }
            UnspannedAtomicToken::Number { number } => {
                Expression::number(number.to_number(context.source()), span)
            }
            UnspannedAtomicToken::String { body } => Expression::string(*body, span),
            UnspannedAtomicToken::ItVariable { name } => Expression::it_variable(*name, span),
            UnspannedAtomicToken::Variable { name } => Expression::variable(*name, span),
            UnspannedAtomicToken::ExternalWord { .. }
            | UnspannedAtomicToken::GlobPattern { .. }
            | UnspannedAtomicToken::Word { .. }
            | UnspannedAtomicToken::Dot { .. }
            | UnspannedAtomicToken::Operator { .. } => Expression::external_command(span, span),
        })
    }
}

#[derive(Debug, Copy, Clone)]
struct ExternalContinuationShape;

impl ExpandExpression for ExternalContinuationShape {
    fn name(&self) -> &'static str {
        "external argument"
    }

    fn expand_expr<'a, 'b>(
        &self,
        token_nodes: &'b mut TokensIterator<'a>,
        context: &ExpandContext,
    ) -> Result<Expression, ParseError> {
        let atom = expand_atom(
            token_nodes,
            "external argument",
            context,
            ExpansionRule::new()
                .allow_external_word()
                .treat_size_as_word(),
        )?;

        let span = atom.span;

        Ok(match &atom.unspanned {
            UnspannedAtomicToken::Eof { .. } => unreachable!("ExpansionRule doesn't allow EOF"),
            UnspannedAtomicToken::Error { .. } => unreachable!("ExpansionRule doesn't allow Error"),
            UnspannedAtomicToken::Number { number } => {
                Expression::number(number.to_number(context.source()), span)
            }
            UnspannedAtomicToken::Size { .. } => unreachable!("ExpansionRule treats size as word"),
            UnspannedAtomicToken::ExternalCommand { .. } => {
                unreachable!("ExpansionRule doesn't allow ExternalCommand")
            }
            UnspannedAtomicToken::Whitespace { .. } => {
                unreachable!("ExpansionRule doesn't allow Whitespace")
            }
            UnspannedAtomicToken::String { body } => Expression::string(*body, span),
            UnspannedAtomicToken::ItVariable { name } => Expression::it_variable(*name, span),
            UnspannedAtomicToken::Variable { name } => Expression::variable(*name, span),
            UnspannedAtomicToken::ExternalWord { .. }
            | UnspannedAtomicToken::GlobPattern { .. }
            | UnspannedAtomicToken::Word { .. }
            | UnspannedAtomicToken::ShorthandFlag { .. }
            | UnspannedAtomicToken::Dot { .. }
            | UnspannedAtomicToken::Operator { .. } => Expression::bare(span),
            UnspannedAtomicToken::SquareDelimited { .. } => {
                return Err(ParseError::mismatch(
                    "external argument",
                    "pipeline".spanned(atom.span),
                ))
            }
        })
    }
}

#[cfg(coloring_in_tokens)]
impl ColorSyntax for ExternalExpression {
    type Info = ExternalExpressionResult;
    type Input = ();

    fn name(&self) -> &'static str {
        "ExternalExpression"
    }

    fn color_syntax<'a, 'b>(
        &self,
        _input: &(),
        token_nodes: &'b mut TokensIterator<'a>,
        context: &ExpandContext,
    ) -> ExternalExpressionResult {
        let atom = match expand_atom(
            token_nodes,
            "external word",
            context,
            ExpansionRule::permissive(),
        ) {
            Err(_) => unreachable!("TODO: separate infallible expand_atom"),
            Ok(AtomicToken {
                unspanned: UnspannedAtomicToken::Eof { .. },
                ..
            }) => return ExternalExpressionResult::Eof,
            Ok(atom) => atom,
        };

        token_nodes.mutate_shapes(|shapes| atom.color_tokens(shapes));
        return ExternalExpressionResult::Processed;
    }
}

#[must_use]
enum ExternalExpressionResult {
    Eof,
    Processed,
}

#[cfg(not(coloring_in_tokens))]
impl ColorSyntax for ExternalExpression {
    type Info = ExternalExpressionResult;
    type Input = ();

    fn color_syntax<'a, 'b>(
        &self,
        _input: &(),
        token_nodes: &'b mut TokensIterator<'a>,
        context: &ExpandContext,
        shapes: &mut Vec<Spanned<FlatShape>>,
    ) -> ExternalExpressionResult {
        let atom = match expand_atom(
            token_nodes,
            "external word",
            context,
            ExpansionRule::permissive(),
        ) {
            Err(_) => unreachable!("TODO: separate infallible expand_atom"),
            Ok(AtomicToken {
                unspanned: UnspannedAtomicToken::Eof { .. },
                ..
            }) => return ExternalExpressionResult::Eof,
            Ok(atom) => atom,
        };

        atom.color_tokens(shapes);
        return ExternalExpressionResult::Processed;
    }
}
