use crate::commands::classified::{internal::InternalCommand, ClassifiedCommand};
use crate::hir::expand_external_tokens::{ExternalTokensShape, ExternalTokensSyntax};
use crate::hir::{
    self, named::NamedValue, syntax_shape::*, Expression, NamedArguments, SpannedExpression,
    TokensIterator,
};
use crate::parse::files::Files;
use crate::parse::token_tree_builder::{CurriedToken, TokenTreeBuilder as b};
use crate::SpannedToken;
use derive_new::new;
use indexmap::IndexMap;
use nu_errors::{ParseError, ShellError};
use nu_protocol::{outln, PathMember, Signature, SyntaxShape};
use nu_source::{HasSpan, PrettyDebugWithSource, Span, SpannedItem, Tag, Text};
use pretty_assertions::assert_eq;
use std::fmt::Debug;

#[test]
fn test_parse_external() {
    parse_tokens(
        fallible(ExternalTokensShape),
        "5kb",
        vec![b::bare("5kb")],
        |tokens| {
            ExternalTokensSyntax::new(
                vec![format!("5kb").spanned(tokens[0].span())].spanned(tokens[0].span()),
            )
        },
    );

    parse_tokens(
        fallible(ExternalTokensShape),
        "cargo +nightly run -- --features all",
        vec![
            b::bare("cargo"),
            b::sp(),
            b::external_word("+nightly"),
            b::sp(),
            b::bare("run"),
            b::sp(),
            b::external_word("--"),
            b::sp(),
            b::flag("features"),
            b::sp(),
            b::bare("all"),
        ],
        |tokens| {
            let cargo = format!("cargo").spanned(tokens[0].span());
            let nightly = format!("+nightly").spanned(tokens[2].span());
            let run = format!("run").spanned(tokens[4].span());
            let dashdash = format!("--").spanned(tokens[6].span());
            let features = format!("--features").spanned(tokens[8].span());
            let all = format!("all").spanned(tokens[10].span());
            let span = tokens[0].span().until(tokens[10].span());

            ExternalTokensSyntax::new(
                vec![cargo, nightly, run, dashdash, features, all].spanned(span),
            )
        },
    );
}

#[test]
fn test_parse_string() {
    parse_tokens(
        CoerceStringShape,
        r#""hello""#,
        vec![b::string("hello")],
        |tokens| {
            Expression::string(inner_string_span(tokens[0].span())).into_expr(tokens[0].span())
        },
    );
}

#[test]
fn test_parse_path() {
    let _ = pretty_env_logger::try_init();

    parse_expr(
        AnyExpressionShape,
        "$it.cpu",
        vec![b::it_var(), b::dot(), b::bare("cpu")],
        |tokens| {
            let (outer_var, inner_var) = tokens[0].expect_var();
            let bare = tokens[2].expect_bare();
            Expression::path(
                Expression::it_variable(inner_var).into_expr(outer_var),
                vec![PathMember::string("cpu", bare)],
            )
            .into_expr(outer_var.until(bare))
        },
    );

    parse_expr(
        VariablePathShape,
        r#"$cpu.amount."max ghz""#,
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

            Expression::path(
                Expression::variable(inner_var).into_expr(outer_var),
                vec![
                    PathMember::string("amount", amount),
                    PathMember::string("max ghz", outer_max_ghz),
                ],
            )
            .into_expr(outer_var.until(outer_max_ghz))
        },
    );
}

#[test]
fn test_parse_command() {
    parse_tokens(
        fallible(ClassifiedCommandShape),
        "ls *.txt",
        vec![b::bare("ls"), b::sp(), b::pattern("*.txt")],
        |tokens| {
            let bare = tokens[0].expect_bare();
            let pat = tokens[2].expect_pattern();

            let mut map = IndexMap::new();
            map.insert("full".to_string(), NamedValue::AbsentSwitch);
            map.insert("help".to_string(), NamedValue::AbsentSwitch);

            ClassifiedCommand::Internal(InternalCommand::new(
                "ls".to_string(),
                Tag {
                    span: bare,
                    anchor: None,
                },
                hir::Call {
                    head: Box::new(Expression::Command(bare).into_expr(bare)),
                    positional: Some(vec![Expression::pattern("*.txt").into_expr(pat)]),
                    named: Some(NamedArguments { named: map }),
                    span: bare.until(pat),
                },
            ))
        },
    );
}

#[derive(Debug, Clone, new)]
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
            .switch(
                "full",
                "list all available columns for each entry",
                Some('f'),
            ),
    );

    callback(ExpandContext::new(Box::new(registry), source, None))
}

trait Expand {}

fn parse_tokens<T: Eq + HasSpan + PrettyDebugWithSource + Clone + Debug + 'static>(
    shape: impl ExpandSyntax<Output = Result<T, ParseError>>,
    syntax: &str,
    tokens: Vec<CurriedToken>,
    expected: impl FnOnce(&[SpannedToken]) -> T,
) {
    // let parsed_tokens = parse(syntax);
    let tokens = b::token_list(tokens);
    let (tokens, source) = b::build(tokens);
    let text = Text::from(&source);

    assert_eq!(syntax, source);

    with_empty_context(&text, |context| {
        let tokens = tokens.expect_list();
        let mut iterator = TokensIterator::new(&tokens.item, context, tokens.span);

        let expr = iterator.expand_syntax(shape);

        let expr = match expr {
            Ok(expr) => expr,
            Err(err) => {
                outln!("");
                ptree::print_tree(&iterator.expand_tracer().print(text.clone())).unwrap();
                outln!("");

                print_err(err.into(), &iterator.context().source().clone());
                panic!("Parse failed");
            }
        };

        let expected = expected(&tokens.item);

        if expr != expected {
            outln!("");
            ptree::print_tree(&iterator.expand_tracer().print(text.clone())).unwrap();
            outln!("");

            assert_eq!(expr, expected);
        }
    })
}

fn parse_expr(
    shape: impl ExpandSyntax<Output = Result<SpannedExpression, ParseError>>,
    syntax: &str,
    tokens: Vec<CurriedToken>,
    expected: impl FnOnce(&[SpannedToken]) -> SpannedExpression,
) {
    // let parsed_tokens = parse(syntax);
    let tokens = b::token_list(tokens);
    let (tokens, source) = b::build(tokens);
    let text = Text::from(&source);

    assert_eq!(syntax, source);

    with_empty_context(&text, |context| {
        let tokens = tokens.expect_list();
        let mut iterator = TokensIterator::new(&tokens.item, context, tokens.span);

        let expr = iterator.expand_syntax(shape);

        let expr = match expr {
            Ok(expr) => expr,
            Err(err) => {
                outln!("");
                ptree::print_tree(&iterator.expand_tracer().print(text.clone())).unwrap();
                outln!("");

                print_err(err.into(), &iterator.source());
                panic!("Parse failed");
            }
        };

        let expected = expected(&tokens.item);

        if expr != expected {
            outln!("");
            ptree::print_tree(&iterator.expand_tracer().print(text.clone())).unwrap();
            outln!("");

            assert_eq!(expr, expected);
        }
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
