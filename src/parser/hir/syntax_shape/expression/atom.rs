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

#[derive(Debug)]
pub enum AtomicToken<'tokens> {
    Eof {
        tag: Tag,
    },
    Error {
        error: Tagged<ShellError>,
    },
    Number {
        number: RawNumber,
    },
    Size {
        number: Tagged<RawNumber>,
        unit: Tagged<Unit>,
    },
    String {
        body: Tag,
    },
    ItVariable {
        name: Tag,
    },
    Variable {
        name: Tag,
    },
    ExternalCommand {
        command: Tag,
    },
    ExternalWord {
        text: Tag,
    },
    GlobPattern {
        pattern: Tag,
    },
    FilePath {
        path: Tag,
    },
    Word {
        text: Tag,
    },
    SquareDelimited {
        tags: (Tag, Tag),
        nodes: &'tokens Vec<TokenNode>,
    },
    ParenDelimited {
        tags: (Tag, Tag),
        nodes: &'tokens Vec<TokenNode>,
    },
    BraceDelimited {
        tags: (Tag, Tag),
        nodes: &'tokens Vec<TokenNode>,
    },
    Pipeline {
        pipe: Option<Tag>,
        elements: Tagged<&'tokens Vec<TokenNode>>,
    },
    ShorthandFlag {
        name: Tag,
    },
    LonghandFlag {
        name: Tag,
    },
    Dot {
        text: Tag,
    },
    Operator {
        text: Tag,
    },
    Whitespace {
        text: Tag,
    },
}

pub type TaggedAtomicToken<'tokens> = Tagged<AtomicToken<'tokens>>;

impl<'tokens> TaggedAtomicToken<'tokens> {
    pub fn into_hir(
        &self,
        context: &ExpandContext,
        expected: &'static str,
    ) -> Result<hir::Expression, ShellError> {
        Ok(match &self.item {
            AtomicToken::Eof { .. } => {
                return Err(ShellError::type_error(
                    expected,
                    "eof atomic token".tagged(self.tag),
                ))
            }
            AtomicToken::Error { .. } => {
                return Err(ShellError::type_error(
                    expected,
                    "eof atomic token".tagged(self.tag),
                ))
            }
            AtomicToken::Operator { .. } => {
                return Err(ShellError::type_error(
                    expected,
                    "operator".tagged(self.tag),
                ))
            }
            AtomicToken::ShorthandFlag { .. } => {
                return Err(ShellError::type_error(
                    expected,
                    "shorthand flag".tagged(self.tag),
                ))
            }
            AtomicToken::LonghandFlag { .. } => {
                return Err(ShellError::type_error(expected, "flag".tagged(self.tag)))
            }
            AtomicToken::Whitespace { .. } => {
                return Err(ShellError::unimplemented("whitespace in AtomicToken"))
            }
            AtomicToken::Dot { .. } => {
                return Err(ShellError::type_error(expected, "dot".tagged(self.tag)))
            }
            AtomicToken::Number { number } => {
                Expression::number(number.to_number(context.source), self.tag)
            }
            AtomicToken::FilePath { path } => Expression::file_path(
                expand_file_path(path.slice(context.source), context),
                self.tag,
            ),
            AtomicToken::Size { number, unit } => {
                Expression::size(number.to_number(context.source), **unit, self.tag)
            }
            AtomicToken::String { body } => Expression::string(body, self.tag),
            AtomicToken::ItVariable { name } => Expression::it_variable(name, self.tag),
            AtomicToken::Variable { name } => Expression::variable(name, self.tag),
            AtomicToken::ExternalCommand { command } => {
                Expression::external_command(command, self.tag)
            }
            AtomicToken::ExternalWord { text } => Expression::string(text, self.tag),
            AtomicToken::GlobPattern { pattern } => Expression::pattern(pattern),
            AtomicToken::Word { text } => Expression::string(text, text),
            AtomicToken::SquareDelimited { .. } => unimplemented!("into_hir"),
            AtomicToken::ParenDelimited { .. } => unimplemented!("into_hir"),
            AtomicToken::BraceDelimited { .. } => unimplemented!("into_hir"),
            AtomicToken::Pipeline { .. } => unimplemented!("into_hir"),
        })
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
        .tagged(self.tag)
    }

    pub(crate) fn color_tokens(&self, shapes: &mut Vec<Tagged<FlatShape>>) {
        match &self.item {
            AtomicToken::Eof { .. } => {}
            AtomicToken::Error { .. } => return shapes.push(FlatShape::Error.tagged(self.tag)),
            AtomicToken::Operator { .. } => {
                return shapes.push(FlatShape::Operator.tagged(self.tag));
            }
            AtomicToken::ShorthandFlag { .. } => {
                return shapes.push(FlatShape::ShorthandFlag.tagged(self.tag));
            }
            AtomicToken::LonghandFlag { .. } => {
                return shapes.push(FlatShape::Flag.tagged(self.tag));
            }
            AtomicToken::Whitespace { .. } => {
                return shapes.push(FlatShape::Whitespace.tagged(self.tag));
            }
            AtomicToken::FilePath { .. } => return shapes.push(FlatShape::Path.tagged(self.tag)),
            AtomicToken::Dot { .. } => return shapes.push(FlatShape::Dot.tagged(self.tag)),
            AtomicToken::Number {
                number: RawNumber::Decimal(_),
            } => {
                return shapes.push(FlatShape::Decimal.tagged(self.tag));
            }
            AtomicToken::Number {
                number: RawNumber::Int(_),
            } => {
                return shapes.push(FlatShape::Int.tagged(self.tag));
            }
            AtomicToken::Size { number, unit } => {
                return shapes.push(
                    FlatShape::Size {
                        number: number.tag,
                        unit: unit.tag,
                    }
                    .tagged(self.tag),
                );
            }
            AtomicToken::String { .. } => return shapes.push(FlatShape::String.tagged(self.tag)),
            AtomicToken::ItVariable { .. } => {
                return shapes.push(FlatShape::ItVariable.tagged(self.tag))
            }
            AtomicToken::Variable { .. } => {
                return shapes.push(FlatShape::Variable.tagged(self.tag))
            }
            AtomicToken::ExternalCommand { .. } => {
                return shapes.push(FlatShape::ExternalCommand.tagged(self.tag));
            }
            AtomicToken::ExternalWord { .. } => {
                return shapes.push(FlatShape::ExternalWord.tagged(self.tag))
            }
            AtomicToken::GlobPattern { .. } => {
                return shapes.push(FlatShape::GlobPattern.tagged(self.tag))
            }
            AtomicToken::Word { .. } => return shapes.push(FlatShape::Word.tagged(self.tag)),
            _ => return shapes.push(FlatShape::Error.tagged(self.tag)),
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

/// If the caller of expand_atom throws away the returned atomic token returned, it
/// must use a checkpoint to roll it back.
pub fn expand_atom<'me, 'content>(
    token_nodes: &'me mut TokensIterator<'content>,
    expected: &'static str,
    context: &ExpandContext,
    rule: ExpansionRule,
) -> Result<TaggedAtomicToken<'content>, ShellError> {
    if token_nodes.at_end() {
        match rule.allow_eof {
            true => {
                return Ok(AtomicToken::Eof {
                    tag: Tag::unknown(),
                }
                .tagged_unknown())
            }
            false => return Err(ShellError::unexpected_eof("anything", Tag::unknown())),
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
            Ok(Tagged {
                item: (number, unit),
                tag,
            }) => return Ok(AtomicToken::Size { number, unit }.tagged(tag)),
        },
    }

    // Try to parse the head of the stream as a bare path. A bare path includes
    // words as well as `.`s, connected together without whitespace.
    match expand_syntax(&BarePathShape, token_nodes, context) {
        // If we didn't find a bare path
        Err(_) => {}
        Ok(tag) => {
            let next = token_nodes.peek_any();

            match next.node {
                Some(token) if token.is_pattern() => {
                    // if the very next token is a pattern, we're looking at a glob, not a
                    // word, and we should try to parse it as a glob next
                }

                _ => return Ok(AtomicToken::Word { text: tag }.tagged(tag)),
            }
        }
    }

    // Try to parse the head of the stream as a pattern. A pattern includes
    // words, words with `*` as well as `.`s, connected together without whitespace.
    match expand_syntax(&BarePatternShape, token_nodes, context) {
        // If we didn't find a bare path
        Err(_) => {}
        Ok(tag) => return Ok(AtomicToken::GlobPattern { pattern: tag }.tagged(tag)),
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
            .tagged(error.tag));
        }

        // [ ... ]
        TokenNode::Delimited(Tagged {
            item:
                DelimitedNode {
                    delimiter: Delimiter::Square,
                    tags,
                    children,
                },
            tag,
        }) => {
            peeked.commit();
            return Ok(AtomicToken::SquareDelimited {
                nodes: children,
                tags: *tags,
            }
            .tagged(tag));
        }

        TokenNode::Flag(Tagged {
            item:
                Flag {
                    kind: FlagKind::Shorthand,
                    name,
                },
            tag,
        }) => {
            peeked.commit();
            return Ok(AtomicToken::ShorthandFlag { name: *name }.tagged(tag));
        }

        TokenNode::Flag(Tagged {
            item:
                Flag {
                    kind: FlagKind::Longhand,
                    name,
                },
            tag,
        }) => {
            peeked.commit();
            return Ok(AtomicToken::ShorthandFlag { name: *name }.tagged(tag));
        }

        // If we see whitespace, process the whitespace according to the whitespace
        // handling rules
        TokenNode::Whitespace(tag) => match rule.whitespace {
            // if whitespace is allowed, return a whitespace token
            WhitespaceHandling::AllowWhitespace => {
                peeked.commit();
                return Ok(AtomicToken::Whitespace { text: *tag }.tagged(tag));
            }

            // if whitespace is disallowed, return an error
            WhitespaceHandling::RejectWhitespace => {
                return Err(ShellError::syntax_error(
                    "Unexpected whitespace".tagged(tag),
                ))
            }
        },

        other => {
            let tag = peeked.node.tag();

            peeked.commit();
            return Ok(AtomicToken::Error {
                error: ShellError::type_error("token", other.tagged_type_name()).tagged(tag),
            }
            .tagged(tag));
        }
    }

    parse_single_node(token_nodes, expected, |token, token_tag, err| {
        Ok(match token {
            // First, the error cases. Each error case corresponds to a expansion rule
            // flag that can be used to allow the case

            // rule.allow_operator
            RawToken::Operator(_) if !rule.allow_operator => return Err(err.error()),
            // rule.allow_external_command
            RawToken::ExternalCommand(_) if !rule.allow_external_command => {
                return Err(ShellError::type_error(
                    expected,
                    token.type_name().tagged(token_tag),
                ))
            }
            // rule.allow_external_word
            RawToken::ExternalWord if !rule.allow_external_word => {
                return Err(ShellError::invalid_external_word(token_tag))
            }

            RawToken::Number(number) => AtomicToken::Number { number }.tagged(token_tag),
            RawToken::Operator(_) => AtomicToken::Operator { text: token_tag }.tagged(token_tag),
            RawToken::String(body) => AtomicToken::String { body }.tagged(token_tag),
            RawToken::Variable(name) if name.slice(context.source) == "it" => {
                AtomicToken::ItVariable { name }.tagged(token_tag)
            }
            RawToken::Variable(name) => AtomicToken::Variable { name }.tagged(token_tag),
            RawToken::ExternalCommand(command) => {
                AtomicToken::ExternalCommand { command }.tagged(token_tag)
            }
            RawToken::ExternalWord => {
                AtomicToken::ExternalWord { text: token_tag }.tagged(token_tag)
            }
            RawToken::GlobPattern => {
                AtomicToken::GlobPattern { pattern: token_tag }.tagged(token_tag)
            }
            RawToken::Bare => AtomicToken::Word { text: token_tag }.tagged(token_tag),
        })
    })
}
