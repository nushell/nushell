use crate::commands::classified::InternalCommand;
use crate::commands::ClassifiedCommand;
use crate::env::host::BasicHost;
use crate::parser::hir::TokensIterator;
use crate::parser::hir::{
    self, named::NamedValue, path::PathMember, syntax_shape::*, NamedArguments,
};
use crate::parser::parse::token_tree_builder::{CurriedToken, TokenTreeBuilder as b};
use crate::parser::TokenNode;
use crate::{HasSpan, Span, SpannedItem, Tag, Text};
use indexmap::IndexMap;
use pretty_assertions::assert_eq;
use std::fmt::Debug;

#[test]
fn test_parse_string() {
    parse_tokens(StringShape, vec![b::string("hello")], |tokens| {
        hir::Expression::string(inner_string_span(tokens[0].span()), tokens[0].span())
    });
}

#[test]
fn test_parse_path() {
    parse_tokens(
        VariablePathShape,
        vec![b::var("it"), b::op("."), b::bare("cpu")],
        |tokens| {
            let (outer_var, inner_var) = tokens[0].expect_var();
            let bare = tokens[2].expect_bare();
            hir::Expression::path(
                hir::Expression::it_variable(inner_var, outer_var),
                vec![PathMember::string("cpu", bare)],
                outer_var.until(bare),
            )
        },
    );

    parse_tokens(
        VariablePathShape,
        vec![
            b::var("cpu"),
            b::op("."),
            b::bare("amount"),
            b::op("."),
            b::string("max ghz"),
        ],
        |tokens| {
            let (outer_var, inner_var) = tokens[0].expect_var();
            let amount = tokens[2].expect_bare();
            let (outer_max_ghz, _) = tokens[4].expect_string();

            hir::Expression::path(
                hir::Expression::variable(inner_var, outer_var),
                vec![
                    PathMember::string("amount", amount),
                    PathMember::string("max ghz", outer_max_ghz),
                ],
                outer_var.until(outer_max_ghz),
            )
        },
    );
}

#[test]
fn test_parse_command() {
    parse_tokens(
        ClassifiedCommandShape,
        vec![b::bare("ls"), b::sp(), b::pattern("*.txt")],
        |tokens| {
            let bare = tokens[0].expect_bare();
            let pat = tokens[2].expect_pattern();

            let mut map = IndexMap::new();
            map.insert("full".to_string(), NamedValue::AbsentSwitch);

            ClassifiedCommand::Internal(InternalCommand::new(
                "ls".to_string(),
                Tag {
                    span: bare,
                    anchor: None,
                },
                hir::Call {
                    head: Box::new(hir::RawExpression::Command(bare).spanned(bare)),
                    positional: Some(vec![hir::Expression::pattern("*.txt", pat)]),
                    named: Some(NamedArguments { named: map }),
                }
                .spanned(bare.until(pat)),
            ))
            // hir::Expression::path(
            //     hir::Expression::variable(inner_var, outer_var),
            //     vec!["cpu".tagged(bare)],
            //     outer_var.until(bare),
            // )
        },
    );
}

fn parse_tokens<T: Eq + HasSpan + Clone + Debug + 'static>(
    shape: impl ExpandSyntax<Output = T>,
    tokens: Vec<CurriedToken>,
    expected: impl FnOnce(&[TokenNode]) -> T,
) {
    let tokens = b::token_list(tokens);
    let (tokens, source) = b::build(tokens);

    ExpandContext::with_empty(&Text::from(source), |context| {
        let tokens = tokens.expect_list();
        let mut iterator = TokensIterator::all(tokens.item, tokens.span);

        let expr = expand_syntax(&shape, &mut iterator, &context);

        let expr = match expr {
            Ok(expr) => expr,
            Err(err) => {
                crate::cli::print_err(err.into(), &BasicHost, context.source().clone());
                panic!("Parse failed");
            }
        };

        assert_eq!(expr, expected(tokens.item));
    })
}

fn inner_string_span(span: Span) -> Span {
    Span::new(span.start() + 1, span.end() - 1)
}
