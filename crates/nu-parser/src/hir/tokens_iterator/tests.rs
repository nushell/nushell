use crate::hir::{syntax_shape::ExpandContext, syntax_shape::SignatureRegistry, TokensIterator};
use crate::parse::token_tree_builder::TokenTreeBuilder as b;
use nu_protocol::Signature;
use nu_source::{Span, Text};

use derive_new::new;

#[derive(Debug, Clone, new)]
struct TestRegistry {
    #[new(default)]
    signatures: indexmap::IndexMap<String, Signature>,
}

impl TestRegistry {}

impl SignatureRegistry for TestRegistry {
    fn has(&self, name: &str) -> bool {
        self.signatures.contains_key(name)
    }
    fn get(&self, name: &str) -> Option<Signature> {
        self.signatures.get(name).cloned()
    }
    fn clone_box(&self) -> Box<dyn SignatureRegistry> {
        Box::new(self.clone())
    }
}

#[test]
fn supplies_tokens() {
    let token = b::it_var();

    let (tokens, source) = b::build(token);

    let tokens = vec![tokens];
    let source = Text::from(&source);

    let mut iterator = TokensIterator::new(
        &tokens,
        ExpandContext::new(Box::new(TestRegistry::new()), &source, None),
        Span::unknown(),
    );

    let token = iterator.next().expect("Token expected.");

    token.expect_var();
}
