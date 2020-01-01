use crate::hir::TokensIterator;
use crate::parse::token_tree_builder::TokenTreeBuilder as b;
use crate::Span;

#[test]
fn supplies_tokens() -> Result<(), Box<dyn std::error::Error>> {
    let tokens = b::token_list(vec![b::var("it"), b::op("."), b::bare("cpu")]);
    let (tokens, _) = b::build(tokens);

    let tokens = tokens.expect_list();
    let mut iterator = TokensIterator::all(tokens, Span::unknown());

    iterator.next()?.expect_var();
    iterator.next()?.expect_dot();
    iterator.next()?.expect_bare();
}
