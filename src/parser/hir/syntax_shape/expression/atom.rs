use crate::parser::hir::syntax_shape::{
    expand_syntax, expression::expand_file_path, parse_single_node, BarePathShape,
    BarePatternShape, ExpandContext, UnitShape,
};
use crate::parser::{
    hir,
    hir::{Expression, RawNumber, TokensIterator},
    parse::flag::{Flag, FlagKind},
    DelimitedNode, Delimiter, FlatShape, RawToken, TokenNode, Unit,
};
use crate::prelude::*;
use crate::{Span, Spanned};

#[derive(Debug, Clone)]
pub enum AtomicToken<'tokens> {
    Eof {
        span: Span,
    },
    Error {
        error: Spanned<ShellError>,
    },
    Number {
        number: RawNumber,
    },
    Size {
        number: Spanned<RawNumber>,
        unit: Spanned<Unit>,
    },
    String {
        body: Span,
    },
    ItVariable {
        name: Span,
    },
    Variable {
        name: Span,
    },
    ExternalCommand {
        command: Span,
    },
    ExternalWord {
        text: Span,
    },
    GlobPattern {
        pattern: Span,
    },
    FilePath {
        path: Span,
    },
    Word {
        text: Span,
    },
    SquareDelimited {
        spans: (Span, Span),
        nodes: &'tokens Vec<TokenNode>,
    },
    ParenDelimited {
        span: (Span, Span),
        nodes: &'tokens Vec<TokenNode>,
    },
    BraceDelimited {
        spans: (Span, Span),
        nodes: &'tokens Vec<TokenNode>,
    },
    Pipeline {
        pipe: Option<Span>,
        elements: Spanned<&'tokens Vec<TokenNode>>,
    },
    ShorthandFlag {
        name: Span,
    },
    LonghandFlag {
        name: Span,
    },
    Dot {
        text: Span,
    },
    Operator {
        text: Span,
    },
    Whitespace {
        text: Span,
    },
}

impl<'tokens> ShellTypeName for AtomicToken<'tokens> {
    fn type_name(&self) -> &'static str {
        match &self {
            AtomicToken::Eof { .. } => "eof",
            AtomicToken::Error { .. } => "error",
            AtomicToken::Operator { .. } => "operator",
            AtomicToken::ShorthandFlag { .. } => "shorthand flag",
            AtomicToken::LonghandFlag { .. } => "flag",
            AtomicToken::Whitespace { .. } => "whitespace",
            AtomicToken::Dot { .. } => "dot",
            AtomicToken::Number { .. } => "number",
            AtomicToken::FilePath { .. } => "file path",
            AtomicToken::Size { .. } => "size",
            AtomicToken::String { .. } => "string",
            AtomicToken::ItVariable { .. } => "$it",
            AtomicToken::Variable { .. } => "variable",
            AtomicToken::ExternalCommand { .. } => "external command",
            AtomicToken::ExternalWord { .. } => "external word",
            AtomicToken::GlobPattern { .. } => "file pattern",
            AtomicToken::Word { .. } => "word",
            AtomicToken::SquareDelimited { .. } => "array literal",
            AtomicToken::ParenDelimited { .. } => "parenthesized expression",
            AtomicToken::BraceDelimited { .. } => "block",
            AtomicToken::Pipeline { .. } => "pipeline",
        }
    }
}

pub type SpannedAtomicToken<'tokens> = Spanned<AtomicToken<'tokens>>;

impl<'tokens> SpannedAtomicToken<'tokens> {
    pub fn into_hir(
        &self,
        context: &ExpandContext,
        expected: &'static str,
    ) -> Result<hir::Expression, ParseError> {
        Ok(match &self.item {
            AtomicToken::Eof { .. } => {
                return Err(ParseError::mismatch(
                    expected,
                    "eof atomic token".spanned(self.span),
                ))
            }
            AtomicToken::Error { .. } => {
                return Err(ParseError::mismatch(
                    expected,
                    "eof atomic token".spanned(self.span),
                ))
            }
            AtomicToken::Operator { .. } => {
                return Err(ParseError::mismatch(
                    expected,
                    "operator".spanned(self.span),
                ))
            }
            AtomicToken::ShorthandFlag { .. } => {
                return Err(ParseError::mismatch(
                    expected,
                    "shorthand flag".spanned(self.span),
                ))
            }
            AtomicToken::LonghandFlag { .. } => {
                return Err(ParseError::mismatch(expected, "flag".spanned(self.span)))
            }
            AtomicToken::Whitespace { .. } => {
                return Err(ParseError::mismatch(
                    expected,
                    "whitespace".spanned(self.span),
                ))
            }
            AtomicToken::Dot { .. } => {
                return Err(ParseError::mismatch(expected, "dot".spanned(self.span)))
            }
            AtomicToken::Number { number } => {
                Expression::number(number.to_number(context.source), self.span)
            }
            AtomicToken::FilePath { path } => Expression::file_path(
                expand_file_path(path.slice(context.source), context),
                self.span,
            ),
            AtomicToken::Size { number, unit } => {
                Expression::size(number.to_number(context.source), **unit, self.span)
            }
            AtomicToken::String { body } => Expression::string(*body, self.span),
            AtomicToken::ItVariable { name } => Expression::it_variable(*name, self.span),
            AtomicToken::Variable { name } => Expression::variable(*name, self.span),
            AtomicToken::ExternalCommand { command } => {
                Expression::external_command(*command, self.span)
            }
            AtomicToken::ExternalWord { text } => Expression::string(*text, self.span),
            AtomicToken::GlobPattern { pattern } => Expression::pattern(
                expand_file_path(pattern.slice(context.source), context).to_string_lossy(),
                self.span,
            ),
            AtomicToken::Word { text } => Expression::string(*text, *text),
            AtomicToken::SquareDelimited { .. } => unimplemented!("into_hir"),
            AtomicToken::ParenDelimited { .. } => unimplemented!("into_hir"),
            AtomicToken::BraceDelimited { .. } => unimplemented!("into_hir"),
            AtomicToken::Pipeline { .. } => unimplemented!("into_hir"),
        })
    }

    pub fn spanned_type_name(&self) -> Spanned<&'static str> {
        match &self.item {
            AtomicToken::Eof { .. } => "eof",
            AtomicToken::Error { .. } => "error",
            AtomicToken::Operator { .. } => "operator",
            AtomicToken::ShorthandFlag { .. } => "shorthand flag",
            AtomicToken::LonghandFlag { .. } => "flag",
            AtomicToken::Whitespace { .. } => "whitespace",
            AtomicToken::Dot { .. } => "dot",
            AtomicToken::Number { .. } => "number",
            AtomicToken::FilePath { .. } => "file path",
            AtomicToken::Size { .. } => "size",
            AtomicToken::String { .. } => "string",
            AtomicToken::ItVariable { .. } => "$it",
            AtomicToken::Variable { .. } => "variable",
            AtomicToken::ExternalCommand { .. } => "external command",
            AtomicToken::ExternalWord { .. } => "external word",
            AtomicToken::GlobPattern { .. } => "file pattern",
            AtomicToken::Word { .. } => "word",
            AtomicToken::SquareDelimited { .. } => "array literal",
            AtomicToken::ParenDelimited { .. } => "parenthesized expression",
            AtomicToken::BraceDelimited { .. } => "block",
            AtomicToken::Pipeline { .. } => "pipeline",
        }
        .spanned(self.span)
    }

    pub fn tagged_type_name(&self) -> Tagged<&'static str> {
        match &self.item {
            AtomicToken::Eof { .. } => "eof",
            AtomicToken::Error { .. } => "error",
            AtomicToken::Operator { .. } => "operator",
            AtomicToken::ShorthandFlag { .. } => "shorthand flag",
            AtomicToken::LonghandFlag { .. } => "flag",
            AtomicToken::Whitespace { .. } => "whitespace",
            AtomicToken::Dot { .. } => "dot",
            AtomicToken::Number { .. } => "number",
            AtomicToken::FilePath { .. } => "file path",
            AtomicToken::Size { .. } => "size",
            AtomicToken::String { .. } => "string",
            AtomicToken::ItVariable { .. } => "$it",
            AtomicToken::Variable { .. } => "variable",
            AtomicToken::ExternalCommand { .. } => "external command",
            AtomicToken::ExternalWord { .. } => "external word",
            AtomicToken::GlobPattern { .. } => "file pattern",
            AtomicToken::Word { .. } => "word",
            AtomicToken::SquareDelimited { .. } => "array literal",
            AtomicToken::ParenDelimited { .. } => "parenthesized expression",
            AtomicToken::BraceDelimited { .. } => "block",
            AtomicToken::Pipeline { .. } => "pipeline",
        }
        .tagged(self.span)
    }

    pub(crate) fn color_tokens(&self, shapes: &mut Vec<Spanned<FlatShape>>) {
        match &self.item {
            AtomicToken::Eof { .. } => {}
            AtomicToken::Error { .. } => return shapes.push(FlatShape::Error.spanned(self.span)),
            AtomicToken::Operator { .. } => {
                return shapes.push(FlatShape::Operator.spanned(self.span));
            }
            AtomicToken::ShorthandFlag { .. } => {
                return shapes.push(FlatShape::ShorthandFlag.spanned(self.span));
            }
            AtomicToken::LonghandFlag { .. } => {
                return shapes.push(FlatShape::Flag.spanned(self.span));
            }
            AtomicToken::Whitespace { .. } => {
                return shapes.push(FlatShape::Whitespace.spanned(self.span));
            }
            AtomicToken::FilePath { .. } => return shapes.push(FlatShape::Path.spanned(self.span)),
            AtomicToken::Dot { .. } => return shapes.push(FlatShape::Dot.spanned(self.span)),
            AtomicToken::Number {
                number: RawNumber::Decimal(_),
            } => {
                return shapes.push(FlatShape::Decimal.spanned(self.span));
            }
            AtomicToken::Number {
                number: RawNumber::Int(_),
            } => {
                return shapes.push(FlatShape::Int.spanned(self.span));
            }
            AtomicToken::Size { number, unit } => {
                return shapes.push(
                    FlatShape::Size {
                        number: number.span,
                        unit: unit.span,
                    }
                    .spanned(self.span),
                );
            }
            AtomicToken::String { .. } => return shapes.push(FlatShape::String.spanned(self.span)),
            AtomicToken::ItVariable { .. } => {
                return shapes.push(FlatShape::ItVariable.spanned(self.span))
            }
            AtomicToken::Variable { .. } => {
                return shapes.push(FlatShape::Variable.spanned(self.span))
            }
            AtomicToken::ExternalCommand { .. } => {
                return shapes.push(FlatShape::ExternalCommand.spanned(self.span));
            }
            AtomicToken::ExternalWord { .. } => {
                return shapes.push(FlatShape::ExternalWord.spanned(self.span))
            }
            AtomicToken::GlobPattern { .. } => {
                return shapes.push(FlatShape::GlobPattern.spanned(self.span))
            }
            AtomicToken::Word { .. } => return shapes.push(FlatShape::Word.spanned(self.span)),
            _ => return shapes.push(FlatShape::Error.spanned(self.span)),
        }
    }
}

#[derive(Debug)]
pub enum WhitespaceHandling {
    #[allow(unused)]
    AllowWhitespace,
    RejectWhitespace,
}

#[derive(Debug)]
pub struct ExpansionRule {
    pub(crate) allow_external_command: bool,
    pub(crate) allow_external_word: bool,
    pub(crate) allow_operator: bool,
    pub(crate) allow_eof: bool,
    pub(crate) treat_size_as_word: bool,
    pub(crate) separate_members: bool,
    pub(crate) commit_errors: bool,
    pub(crate) whitespace: WhitespaceHandling,
}

impl ExpansionRule {
    pub fn new() -> ExpansionRule {
        ExpansionRule {
            allow_external_command: false,
            allow_external_word: false,
            allow_operator: false,
            allow_eof: false,
            treat_size_as_word: false,
            separate_members: false,
            commit_errors: false,
            whitespace: WhitespaceHandling::RejectWhitespace,
        }
    }

    /// The intent of permissive mode is to return an atomic token for every possible
    /// input token. This is important for error-correcting parsing, such as the
    /// syntax highlighter.
    pub fn permissive() -> ExpansionRule {
        ExpansionRule {
            allow_external_command: true,
            allow_external_word: true,
            allow_operator: true,
            allow_eof: true,
            separate_members: false,
            treat_size_as_word: false,
            commit_errors: true,
            whitespace: WhitespaceHandling::AllowWhitespace,
        }
    }

    #[allow(unused)]
    pub fn allow_external_command(mut self) -> ExpansionRule {
        self.allow_external_command = true;
        self
    }

    #[allow(unused)]
    pub fn allow_operator(mut self) -> ExpansionRule {
        self.allow_operator = true;
        self
    }

    #[allow(unused)]
    pub fn no_operator(mut self) -> ExpansionRule {
        self.allow_operator = false;
        self
    }

    #[allow(unused)]
    pub fn no_external_command(mut self) -> ExpansionRule {
        self.allow_external_command = false;
        self
    }

    #[allow(unused)]
    pub fn allow_external_word(mut self) -> ExpansionRule {
        self.allow_external_word = true;
        self
    }

    #[allow(unused)]
    pub fn no_external_word(mut self) -> ExpansionRule {
        self.allow_external_word = false;
        self
    }

    #[allow(unused)]
    pub fn treat_size_as_word(mut self) -> ExpansionRule {
        self.treat_size_as_word = true;
        self
    }

    #[allow(unused)]
    pub fn separate_members(mut self) -> ExpansionRule {
        self.separate_members = true;
        self
    }

    #[allow(unused)]
    pub fn no_separate_members(mut self) -> ExpansionRule {
        self.separate_members = false;
        self
    }

    #[allow(unused)]
    pub fn commit_errors(mut self) -> ExpansionRule {
        self.commit_errors = true;
        self
    }

    #[allow(unused)]
    pub fn allow_whitespace(mut self) -> ExpansionRule {
        self.whitespace = WhitespaceHandling::AllowWhitespace;
        self
    }

    #[allow(unused)]
    pub fn reject_whitespace(mut self) -> ExpansionRule {
        self.whitespace = WhitespaceHandling::RejectWhitespace;
        self
    }
}

impl<'content> FormatDebug for SpannedAtomicToken<'content> {
    fn fmt_debug(&self, f: &mut DebugFormatter, source: &str) -> std::fmt::Result {
        match &self.item {
            AtomicToken::Eof { .. } => f.say_str("atomic", "eof"),
            AtomicToken::Error { .. } => f.say_str("atomic", "error"),
            AtomicToken::Number { number } => {
                f.say_str("atomic", format!("{}", number.debug(source)))
            }
            AtomicToken::Size { number, unit } => f.say_str(
                "atomic size",
                format!("{}{}", number.debug(source), unit.debug(source)),
            ),
            AtomicToken::String { body } => f.say_str("atomic string", body.slice(source)),
            AtomicToken::ItVariable { name } => f.say_str("atomic it", name.slice(source)),
            AtomicToken::Variable { name } => f.say_str("atomic variable", name.slice(source)),
            AtomicToken::ExternalCommand { command } => {
                f.say_str("atomic external command", command.slice(source))
            }
            AtomicToken::ExternalWord { text } => {
                f.say_str("atomic external word", text.slice(source))
            }
            AtomicToken::GlobPattern { pattern } => f.say_str("atomic glob", pattern.slice(source)),
            AtomicToken::FilePath { path } => f.say_str("atomic path", path.slice(source)),
            AtomicToken::Word { text } => f.say_str("word", text.slice(source)),
            AtomicToken::SquareDelimited { .. } => f.say_simple("atomic square"),
            AtomicToken::ParenDelimited { .. } => f.say_simple("atomic paren"),
            AtomicToken::BraceDelimited { .. } => f.say_simple("atomic brace"),
            AtomicToken::Pipeline { .. } => f.say_simple("atomic pipeline"),
            AtomicToken::ShorthandFlag { name } => {
                f.say_str("atomic shorthand", name.slice(source))
            }
            AtomicToken::LonghandFlag { name } => f.say_str("atomic longhand", name.slice(source)),
            AtomicToken::Dot { .. } => f.say_simple("atomic dot"),
            AtomicToken::Operator { text } => f.say_str("atomic operator", text.slice(source)),
            AtomicToken::Whitespace { text } => {
                f.say_str("atomic whitespace", &format!("{:?}", text.slice(source)))
            }
        }
    }
}

pub fn expand_atom<'me, 'content>(
    token_nodes: &'me mut TokensIterator<'content>,
    expected: &'static str,
    context: &ExpandContext,
    rule: ExpansionRule,
) -> Result<SpannedAtomicToken<'content>, ParseError> {
    token_nodes.with_expand_tracer(|_, tracer| tracer.start("atom"));

    let result = expand_atom_inner(token_nodes, expected, context, rule);

    token_nodes.with_expand_tracer(|_, tracer| match &result {
        Ok(result) => {
            tracer.add_result(Box::new(format!("{}", result.debug(context.source))));
            tracer.success();
        }

        Err(err) => tracer.failed(err),
    });

    result
}

/// If the caller of expand_atom throws away the returned atomic token returned, it
/// must use a checkpoint to roll it back.
fn expand_atom_inner<'me, 'content>(
    token_nodes: &'me mut TokensIterator<'content>,
    expected: &'static str,
    context: &ExpandContext,
    rule: ExpansionRule,
) -> Result<SpannedAtomicToken<'content>, ParseError> {
    if token_nodes.at_end() {
        match rule.allow_eof {
            true => {
                return Ok(AtomicToken::Eof {
                    span: Span::unknown(),
                }
                .spanned(Span::unknown()))
            }
            false => return Err(ParseError::unexpected_eof("anything", Span::unknown())),
        }
    }

    // First, we'll need to handle the situation where more than one token corresponds
    // to a single atomic token

    // If treat_size_as_word, don't try to parse the head of the token stream
    // as a size.
    match rule.treat_size_as_word {
        true => {}
        false => match expand_syntax(&UnitShape, token_nodes, context) {
            // If the head of the stream isn't a valid unit, we'll try to parse
            // it again next as a word
            Err(_) => {}

            // But if it was a valid unit, we're done here
            Ok(Spanned {
                item: (number, unit),
                span,
            }) => return Ok(AtomicToken::Size { number, unit }.spanned(span)),
        },
    }

    match rule.separate_members {
        false => {}
        true => {
            let mut next = token_nodes.peek_any();

            match next.node {
                Some(token) if token.is_word() => {
                    next.commit();
                    return Ok(AtomicToken::Word { text: token.span() }.spanned(token.span()));
                }

                Some(token) if token.is_int() => {
                    next.commit();
                    return Ok(AtomicToken::Number {
                        number: RawNumber::Int(token.span()),
                    }
                    .spanned(token.span()));
                }

                _ => {}
            }
        }
    }

    // Try to parse the head of the stream as a bare path. A bare path includes
    // words as well as `.`s, connected together without whitespace.
    match expand_syntax(&BarePathShape, token_nodes, context) {
        // If we didn't find a bare path
        Err(_) => {}
        Ok(span) => {
            let next = token_nodes.peek_any();

            match next.node {
                Some(token) if token.is_pattern() => {
                    // if the very next token is a pattern, we're looking at a glob, not a
                    // word, and we should try to parse it as a glob next
                }

                _ => return Ok(AtomicToken::Word { text: span }.spanned(span)),
            }
        }
    }

    // Try to parse the head of the stream as a pattern. A pattern includes
    // words, words with `*` as well as `.`s, connected together without whitespace.
    match expand_syntax(&BarePatternShape, token_nodes, context) {
        // If we didn't find a bare path
        Err(_) => {}
        Ok(span) => return Ok(AtomicToken::GlobPattern { pattern: span }.spanned(span)),
    }

    // The next token corresponds to at most one atomic token

    // We need to `peek` because `parse_single_node` doesn't cover all of the
    // cases that `expand_atom` covers. We should probably collapse the two
    // if possible.
    let peeked = token_nodes.peek_any().not_eof(expected)?;

    match peeked.node {
        TokenNode::Token(_) => {
            // handle this next
        }

        TokenNode::Error(error) => {
            peeked.commit();
            return Ok(AtomicToken::Error {
                error: error.clone(),
            }
            .spanned(error.span));
        }

        // [ ... ]
        TokenNode::Delimited(Spanned {
            item:
                DelimitedNode {
                    delimiter: Delimiter::Square,
                    spans,
                    children,
                },
            span,
        }) => {
            peeked.commit();
            let span = *span;
            return Ok(AtomicToken::SquareDelimited {
                nodes: children,
                spans: *spans,
            }
            .spanned(span));
        }

        TokenNode::Flag(Spanned {
            item:
                Flag {
                    kind: FlagKind::Shorthand,
                    name,
                },
            span,
        }) => {
            peeked.commit();
            return Ok(AtomicToken::ShorthandFlag { name: *name }.spanned(*span));
        }

        TokenNode::Flag(Spanned {
            item:
                Flag {
                    kind: FlagKind::Longhand,
                    name,
                },
            span,
        }) => {
            peeked.commit();
            return Ok(AtomicToken::ShorthandFlag { name: *name }.spanned(*span));
        }

        // If we see whitespace, process the whitespace according to the whitespace
        // handling rules
        TokenNode::Whitespace(span) => match rule.whitespace {
            // if whitespace is allowed, return a whitespace token
            WhitespaceHandling::AllowWhitespace => {
                peeked.commit();
                return Ok(AtomicToken::Whitespace { text: *span }.spanned(*span));
            }

            // if whitespace is disallowed, return an error
            WhitespaceHandling::RejectWhitespace => {
                return Err(ParseError::mismatch(expected, "whitespace".spanned(*span)))
            }
        },

        other => {
            let span = peeked.node.span();

            peeked.commit();
            return Ok(AtomicToken::Error {
                error: ShellError::type_error("token", other.type_name().spanned(span))
                    .spanned(span),
            }
            .spanned(span));
        }
    }

    parse_single_node(token_nodes, expected, |token, token_span, err| {
        Ok(match token {
            // First, the error cases. Each error case corresponds to a expansion rule
            // flag that can be used to allow the case

            // rule.allow_operator
            RawToken::Operator(_) if !rule.allow_operator => return Err(err.error()),
            // rule.allow_external_command
            RawToken::ExternalCommand(_) if !rule.allow_external_command => {
                return Err(ParseError::mismatch(
                    expected,
                    token.type_name().spanned(token_span),
                ))
            }
            // rule.allow_external_word
            RawToken::ExternalWord if !rule.allow_external_word => {
                return Err(ParseError::mismatch(
                    expected,
                    "external word".spanned(token_span),
                ))
            }

            RawToken::Number(number) => AtomicToken::Number { number }.spanned(token_span),
            RawToken::Operator(_) => AtomicToken::Operator { text: token_span }.spanned(token_span),
            RawToken::String(body) => AtomicToken::String { body }.spanned(token_span),
            RawToken::Variable(name) if name.slice(context.source) == "it" => {
                AtomicToken::ItVariable { name }.spanned(token_span)
            }
            RawToken::Variable(name) => AtomicToken::Variable { name }.spanned(token_span),
            RawToken::ExternalCommand(command) => {
                AtomicToken::ExternalCommand { command }.spanned(token_span)
            }
            RawToken::ExternalWord => {
                AtomicToken::ExternalWord { text: token_span }.spanned(token_span)
            }
            RawToken::GlobPattern => AtomicToken::GlobPattern {
                pattern: token_span,
            }
            .spanned(token_span),
            RawToken::Bare => AtomicToken::Word { text: token_span }.spanned(token_span),
        })
    })
}
