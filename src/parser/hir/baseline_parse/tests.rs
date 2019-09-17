use crate::commands::classified::InternalCommand;
use crate::commands::ClassifiedCommand;
use crate::env::host::BasicHost;
use crate::parser::hir;
use crate::parser::hir::syntax_shape::*;
use crate::parser::hir::TokensIterator;
use crate::parser::parse::token_tree_builder::{CurriedToken, TokenTreeBuilder as b};
use crate::parser::TokenNode;
use crate::{Span, Tag, Tagged, TaggedItem, Text};
use pretty_assertions::assert_eq;
use std::fmt::Debug;
use uuid::Uuid;

#[test]
fn test_parse_string() {
    parse_tokens(StringShape, vec![b::string("hello")], |tokens| {
        hir::Expression::string(inner_string_tag(tokens[0].tag()), tokens[0].tag())
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
                vec!["cpu".tagged(bare)],
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
                vec!["amount".tagged(amount), "max ghz".tagged(outer_max_ghz)],
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
            let pat = tokens[2].tag();

            ClassifiedCommand::Internal(InternalCommand::new(
                "ls".to_string(),
                bare,
                hir::Call {
                    head: Box::new(hir::RawExpression::Command(bare).tagged(bare)),
                    positional: Some(vec![hir::Expression::pattern(pat)]),
                    named: None,
                },
            ))
            // hir::Expression::path(
            //     hir::Expression::variable(inner_var, outer_var),
            //     vec!["cpu".tagged(bare)],
            //     outer_var.until(bare),
            // )
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
                vec!["amount".tagged(amount), "max ghz".tagged(outer_max_ghz)],
                outer_var.until(outer_max_ghz),
            )
        },
    );
}

fn parse_tokens<T: Eq + Debug>(
    shape: impl ExpandSyntax<Output = T>,
    tokens: Vec<CurriedToken>,
    expected: impl FnOnce(Tagged<&[TokenNode]>) -> T,
) {
    let tokens = b::token_list(tokens);
    let (tokens, source) = b::build(test_origin(), tokens);

    ExpandContext::with_empty(&Text::from(source), |context| {
        let tokens = tokens.expect_list();
        let mut iterator = TokensIterator::all(tokens.item, *context.tag());

        let expr = expand_syntax(&shape, &mut iterator, &context);

        let expr = match expr {
            Ok(expr) => expr,
            Err(err) => {
                crate::cli::print_err(err, &BasicHost, context.source().clone());
                panic!("Parse failed");
            }
        };

        assert_eq!(expr, expected(tokens));
    })
}

fn test_origin() -> Uuid {
    Uuid::nil()
}

fn inner_string_tag(tag: Tag) -> Tag {
    Tag {
        span: Span::new(tag.span.start() + 1, tag.span.end() - 1),
        anchor: tag.anchor,
    }
}
