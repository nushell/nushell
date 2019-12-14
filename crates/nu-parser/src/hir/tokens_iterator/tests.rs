use crate::hir::TokensIterator;
use crate::parse::token_tree_builder::TokenTreeBuilder as b;
use crate::Span;

#[test]
fn supplies_tokens() {
    let tokens = b::token_list(vec![b::it_var(), b::op("."), b::bare("cpu")]);
    let (tokens, _) = b::build(tokens);

    let tokens = tokens.expect_list();
    let mut iterator = TokensIterator::new(tokens, Span::unknown());

    iterator.next().unwrap().expect_var();
    iterator.next().unwrap().expect_dot();
    iterator.next().unwrap().expect_bare();
}
