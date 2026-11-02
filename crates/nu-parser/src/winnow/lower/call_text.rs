//! Calls and values read from a statement's items, as nu-parser's readers read them, where the
//! winnow parser read the items otherwise: an external call (`parse_external_call`) and the
//! value of a multi-item shape (`parse_multispan_value`).

use nu_protocol::{
    IntoSpanned, Span, SyntaxShape, Type,
    ast::{Expr, Expression, ExternalArgument},
};

use super::{Lower, Lowered, Unlowered};
use crate::parse_helpers::extract_spread_list;
use crate::winnow::syntax::ast as w;

impl<'s> Lower<'_, '_, 's> {
    /// The value of `shape` read from the items `spans` (`parse_multispan_value` from the
    /// first), each item an argument the winnow parser read nothing of, so read from its text.
    /// Returns it and the index of the item after the last one it read.
    pub(super) fn multispan_value_text(
        &mut self,
        spans: &[Span],
        shape: &SyntaxShape,
        input_type: Option<&Type>,
    ) -> Lowered<(Expression, usize)> {
        let arguments: Vec<w::Argument<'s>> = spans
            .iter()
            .map(|span| {
                let item = w::Expression::new(w::Expr::Garbage, self.local(*span));
                w::Argument::Positional(item)
            })
            .collect();
        self.multispan_value(&arguments, 0, arguments.len(), shape, input_type)
    }

    /// A call to the external command named by the first of the items `spans`, the others its
    /// arguments, covering `call_span` (`parse_external_call`): a head starting with `$` or `(`
    /// is an expression, any other a string; an argument `...x` spreads a list, any other is
    /// read as an external argument.
    pub(super) fn external_call_text(
        &mut self,
        spans: &[Span],
        call_span: Span,
    ) -> Lowered<Expression> {
        let Some((&head_span, args)) = spans.split_first() else {
            return Err(Unlowered::Unsupported("external call without a head"));
        };
        let head_item = self.local(head_span);
        // An expression of one item, which `parse_expression` reads as math.
        let head = match self.text_at(head_span).as_bytes().first() {
            Some(b'$' | b'(') => self.math_items(&[head_item], None, None)?,
            _ => self.external_string_text(head_item)?,
        };
        let mut arguments = Vec::with_capacity(args.len());
        for &arg in args {
            let text = self.text_at(arg).as_bytes();
            let argument = match extract_spread_list(text.into_spanned(arg)) {
                Some(inner) => {
                    let shape = SyntaxShape::List(Box::new(SyntaxShape::Any));
                    ExternalArgument::Spread(self.value_text(self.local(inner.span), &shape)?)
                }
                None => ExternalArgument::Regular(self.regular_external_arg_text(self.local(arg))?),
            };
            arguments.push(argument);
        }
        Ok(self.node(
            Expr::ExternalCall(Box::new(head), arguments.into()),
            call_span,
            Type::Any,
        ))
    }
}
