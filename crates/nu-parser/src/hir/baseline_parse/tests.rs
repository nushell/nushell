use crate::commands::classified::{internal::InternalCommand, ClassifiedCommand};
use crate::hir::TokensIterator;
use crate::hir::{self, named::NamedValue, syntax_shape::*, NamedArguments};
use crate::parse::files::Files;
use crate::parse::token_tree_builder::{CurriedToken, TokenTreeBuilder as b};
use crate::TokenNode;
use derive_new::new;
use indexmap::IndexMap;
use nu_errors::ShellError;
use nu_protocol::{PathMember, Signature, SyntaxShape};
use nu_source::{HasSpan, Span, Tag, Text};
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
        vec![b::var("it"), b::dot(), b::bare("cpu")],
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
            b::dot(),
            b::bare("amount"),
            b::dot(),
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
                    head: Box::new(hir::RawExpression::Command(bare).into_expr(bare)),
                    positional: Some(vec![hir::Expression::pattern("*.txt", pat)]),
                    named: Some(NamedArguments { named: map }),
                    span: bare.until(pat),
                },
            ))
        },
    );
}

#[derive(new)]
struct TestRegistry {
    #[new(default)]
    signatures: indexmap::IndexMap<String, Signature>,
}

impl TestRegistry {
    fn insert(&mut self, key: &str, value: Signature) {
        self.signatures.insert(key.to_string(), value);
    }
}

impl SignatureRegistry for TestRegistry {
    fn has(&self, name: &str) -> Result<bool, ShellError> {
        Ok(self.signatures.contains_key(name))
    }
    fn get(&self, name: &str) -> Result<Option<Signature>, ShellError> {
        Ok(self.signatures.get(name).cloned())
    }
}

fn with_empty_context(source: &Text, callback: impl FnOnce(ExpandContext)) {
    let mut registry = TestRegistry::new();
    registry.insert(
        "ls",
        Signature::build("ls")
            .optional(
                "path",
                SyntaxShape::Pattern,
                "a path to get the directory contents from",
            )
            .switch("full", "list all available columns for each entry"),
    );

    callback(ExpandContext::new(Box::new(registry), source, None))
}

fn parse_tokens<T: Eq + HasSpan + Clone + Debug + 'static>(
    shape: impl ExpandSyntax<Output = T>,
    tokens: Vec<CurriedToken>,
    expected: impl FnOnce(&[TokenNode]) -> T,
) {
    let tokens = b::token_list(tokens);
    let (tokens, source) = b::build(tokens);
    let text = Text::from(source);

    with_empty_context(&text, |context| {
        let tokens = tokens.expect_list();
        let mut iterator = TokensIterator::all(tokens.item, text.clone(), tokens.span);

        let expr = expand_syntax(&shape, &mut iterator, &context);

        let expr = match expr {
            Ok(expr) => expr,
            Err(err) => {
                print_err(err.into(), &context.source().clone());
                panic!("Parse failed");
            }
        };

        assert_eq!(expr, expected(tokens.item));
    })
}

fn inner_string_span(span: Span) -> Span {
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
