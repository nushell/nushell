use crate::hir::{syntax_shape::ExpandContext, syntax_shape::SignatureRegistry};

use crate::parse::files::Files;
use crate::parse::token_tree::{DelimitedNode, Delimiter, SpannedToken, Token};
use crate::parse::token_tree_builder::{CurriedToken, TokenTreeBuilder};

use nu_errors::ShellError;
use nu_protocol::Signature;
use nu_source::{nom_input, NomSpan, Span, Spanned, Text};

pub use nu_source::PrettyDebug;

use derive_new::new;

pub type CurriedNode<T> = Box<dyn FnOnce(&mut TokenTreeBuilder) -> T + 'static>;

#[derive(Debug, Clone, new)]
pub struct TestRegistry {
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

pub fn with_empty_context(source: &Text, callback: impl FnOnce(ExpandContext)) {
    let registry = TestRegistry::new();
    callback(ExpandContext::new(Box::new(registry), source, None))
}

pub fn inner_string_span(span: Span) -> Span {
    Span::new(span.start() + 1, span.end() - 1)
}

pub fn print_err(err: ShellError, source: &Text) {
    let diag = err.into_diagnostic();

    let writer = termcolor::StandardStream::stderr(termcolor::ColorChoice::Auto);
    let mut source = source.to_string();
    source.push_str(" ");
    let files = Files::new(source);
    let _ = language_reporting::emit(
        &mut writer.lock(),
        &files,
        &diag,
        &language_reporting::DefaultConfig,
    );
}

pub fn apply(
    f: impl Fn(NomSpan) -> Result<(NomSpan, SpannedToken), nom::Err<(NomSpan, nom::error::ErrorKind)>>,
    _desc: &str,
    string: &str,
) -> SpannedToken {
    let result = f(nom_input(string));

    match result {
        Ok(value) => value.1,
        Err(err) => {
            let err = nu_errors::ShellError::parse_error(err);

            println!("{:?}", string);
            crate::hir::baseline_parse::tests::print_err(err, &nu_source::Text::from(string));
            panic!("test failed")
        }
    }
}

pub fn span((left, right): (usize, usize)) -> Span {
    Span::new(left, right)
}

pub fn delimited(
    delimiter: Spanned<Delimiter>,
    children: Vec<SpannedToken>,
    left: usize,
    right: usize,
) -> SpannedToken {
    let start = Span::for_char(left);
    let end = Span::for_char(right);

    let node = DelimitedNode::new(delimiter.item, (start, end), children);
    Token::Delimited(node).into_spanned((left, right))
}

pub fn build<T>(block: CurriedNode<T>) -> T {
    let mut builder = TokenTreeBuilder::new();
    block(&mut builder)
}

pub fn build_token(block: CurriedToken) -> SpannedToken {
    TokenTreeBuilder::build(block).0
}
