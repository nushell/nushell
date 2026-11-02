//! nu-parser's lite parse (`lite_parse`), over the winnow lexer's tokens: the tokens of a block
//! grouped into pipelines of commands, each with its items, its redirection and its comments,
//! and the first error nu's lite parser reports. The lowering reads text again through it where
//! nu lite-parses text again (the inside of a list, a record or a closure).

use nu_protocol::{ParseError, Span, ast::RedirectionSource};

use super::Lower;
use crate::winnow::syntax::lex::{RedirectionOperator, Token, TokenContents};

/// Where a redirection sends a stream: a file, or the next command.
#[derive(Clone, Copy, Debug)]
pub(super) enum LiteTarget {
    /// `o> file` and the like.
    File { connector: Span },
    /// `e>|` and `o+e>|`.
    Pipe { connector: Span },
}

impl LiteTarget {
    fn connector(self) -> Span {
        match self {
            LiteTarget::File { connector, .. } | LiteTarget::Pipe { connector } => connector,
        }
    }
}

/// A command's redirection (nu's `LiteRedirection`).
#[derive(Clone, Copy, Debug)]
pub(super) enum LiteRedirection {
    Single {
        source: RedirectionSource,
        target: LiteTarget,
    },
    Separate {
        out: LiteTarget,
        err: LiteTarget,
    },
}

/// One command of a lite-parsed pipeline (nu's `LiteCommand`).
#[derive(Clone, Debug, Default)]
pub(super) struct LiteCommand {
    /// The `|` after it.
    pub pipe: Option<Span>,
    pub comments: Vec<Span>,
    pub parts: Vec<Span>,
    pub redirection: Option<LiteRedirection>,
    /// One past the end of each attribute line's parts.
    pub attribute_idx: Vec<usize>,
}

impl LiteCommand {
    fn accepts_redirection(&self, span: Span) -> Option<ParseError> {
        self.parts
            .is_empty()
            .then_some(ParseError::UnexpectedRedirection { span })
    }

    fn add_redirection(
        &mut self,
        source: RedirectionSource,
        target: LiteTarget,
    ) -> Result<(), ParseError> {
        let redirection = match (self.redirection.take(), source) {
            (None, _) if self.parts.is_empty() => Err(ParseError::UnexpectedRedirection {
                span: target.connector(),
            }),
            (None, source) => Ok(LiteRedirection::Single { source, target }),
            (
                Some(LiteRedirection::Single {
                    source: RedirectionSource::Stdout,
                    target: out,
                }),
                RedirectionSource::Stderr,
            ) => Ok(LiteRedirection::Separate { out, err: target }),
            (
                Some(LiteRedirection::Single {
                    source: RedirectionSource::Stderr,
                    target: err,
                }),
                RedirectionSource::Stdout,
            ) => Ok(LiteRedirection::Separate { out: target, err }),
            (
                Some(LiteRedirection::Single {
                    source,
                    target: first,
                }),
                _,
            ) => Err(ParseError::MultipleRedirections(
                source,
                first.connector(),
                target.connector(),
            )),
            (
                Some(LiteRedirection::Separate { out, .. }),
                RedirectionSource::Stdout | RedirectionSource::StdoutAndStderr,
            ) => Err(ParseError::MultipleRedirections(
                RedirectionSource::Stdout,
                out.connector(),
                target.connector(),
            )),
            (Some(LiteRedirection::Separate { err, .. }), RedirectionSource::Stderr) => {
                Err(ParseError::MultipleRedirections(
                    RedirectionSource::Stderr,
                    err.connector(),
                    target.connector(),
                ))
            }
        }?;
        self.redirection = Some(redirection);
        Ok(())
    }
}

/// A lite-parsed block: its pipelines, each of its commands.
#[derive(Debug, Default)]
pub(super) struct LiteBlock {
    pub pipelines: Vec<Vec<LiteCommand>>,
}

/// The kind of token nu's lite parser tells apart.
#[derive(Clone, Copy, PartialEq, Eq)]
enum Kind {
    Item,
    Comment,
    Pipe,
    PipePipe,
    Semicolon,
    Eol,
    Assignment,
    File {
        source: RedirectionSource,
        append: bool,
    },
    PipeRedirection(RedirectionSource),
}

fn kind(contents: TokenContents) -> Option<Kind> {
    Some(match contents {
        TokenContents::Item => Kind::Item,
        TokenContents::Comment => Kind::Comment,
        TokenContents::Pipe => Kind::Pipe,
        TokenContents::PipePipe => Kind::PipePipe,
        TokenContents::Semicolon => Kind::Semicolon,
        TokenContents::Eol => Kind::Eol,
        TokenContents::AssignmentOperator(_) => Kind::Assignment,
        TokenContents::Redirection(operator) => {
            let (source, append, pipe) = match operator {
                RedirectionOperator::Out => (RedirectionSource::Stdout, false, false),
                RedirectionOperator::OutAppend => (RedirectionSource::Stdout, true, false),
                RedirectionOperator::Err => (RedirectionSource::Stderr, false, false),
                RedirectionOperator::ErrAppend => (RedirectionSource::Stderr, true, false),
                RedirectionOperator::OutErr => (RedirectionSource::StdoutAndStderr, false, false),
                RedirectionOperator::OutErrAppend => {
                    (RedirectionSource::StdoutAndStderr, true, false)
                }
                RedirectionOperator::ErrPipe => (RedirectionSource::Stderr, false, true),
                RedirectionOperator::OutErrPipe => {
                    (RedirectionSource::StdoutAndStderr, false, true)
                }
            };
            match pipe {
                true => Kind::PipeRedirection(source),
                false => Kind::File { source, append },
            }
        }
        TokenContents::Eof => return None,
    })
}

/// The last token before `index` that is no comment, comments on lines of their own skipped
/// (nu's `last_non_comment_token`).
fn last_non_comment(kinds: &[Kind], index: usize) -> Option<Kind> {
    let mut expect = Kind::Comment;
    for kind in kinds[..index].iter().rev() {
        match (*kind, expect) {
            (Kind::Comment, Kind::Comment) | (Kind::Comment, Kind::Eol) => expect = Kind::Eol,
            (Kind::Eol, Kind::Eol) => expect = Kind::Comment,
            (kind, _) => return Some(kind),
        }
    }
    None
}

#[derive(PartialEq, Eq)]
enum Mode {
    Assignment,
    Attribute,
    Normal,
}

impl Lower<'_, '_, '_> {
    /// `tokens` lite-parsed as nu's `lite_parse` does it: commands split at `|`, pipelines at
    /// ends of line and `;`, a redirection and its target taken out of a command's parts, an
    /// assignment taking the rest of the line, attribute lines kept apart; with the first error
    /// nu reports there (a redirection out of place or without its target, `||`, a `|` that
    /// ends the tokens).
    pub(super) fn lite_parse(&self, tokens: &[Token]) -> (LiteBlock, Option<ParseError>) {
        let tokens: Vec<(Kind, Span)> = tokens
            .iter()
            .filter_map(|token| Some((kind(token.contents)?, self.span(token.span))))
            .collect();
        let kinds: Vec<Kind> = tokens.iter().map(|(kind, _)| *kind).collect();
        let mut block = LiteBlock::default();
        if tokens.is_empty() {
            return (block, None);
        }
        let mut pipeline: Vec<LiteCommand> = Vec::new();
        let mut command = LiteCommand::default();
        let mut last = Kind::Eol;
        let mut file_redirection: Option<(RedirectionSource, Span)> = None;
        let mut comment: Option<Vec<Span>> = None;
        let mut mode = Mode::Normal;
        let mut error: Option<ParseError> = None;
        let push_command = |pipeline: &mut Vec<LiteCommand>, command: &mut LiteCommand| {
            if !command.parts.is_empty() || command.redirection.is_some() {
                pipeline.push(std::mem::take(command));
            }
        };
        let push_pipeline = |block: &mut LiteBlock, pipeline: &mut Vec<LiteCommand>| {
            if !pipeline.is_empty() {
                block.pipelines.push(std::mem::take(pipeline));
            }
        };
        for (index, &(kind, span)) in tokens.iter().enumerate() {
            match mode {
                Mode::Attribute => match kind {
                    // An attribute line ends at the end of the line or a `;`.
                    Kind::Eol | Kind::Semicolon => {
                        command.attribute_idx.push(command.parts.len());
                        mode = Mode::Normal;
                        if let Kind::Eol | Kind::Semicolon = last {
                            comment = None;
                            push_command(&mut pipeline, &mut command);
                            push_pipeline(&mut block, &mut pipeline);
                        }
                    }
                    Kind::Comment => {
                        command.comments.push(span);
                        comment = None;
                    }
                    _ => command.parts.push(span),
                },
                // An assignment takes pipes and redirections up to the end of the line or a `;`.
                Mode::Assignment => match kind {
                    Kind::Eol => {
                        if last_non_comment(&kinds, index) != Some(Kind::Pipe) {
                            mode = Mode::Normal;
                            push_command(&mut pipeline, &mut command);
                            push_pipeline(&mut block, &mut pipeline);
                        }
                        if last == Kind::Eol {
                            comment = None;
                        }
                    }
                    Kind::Semicolon => {
                        mode = Mode::Normal;
                        push_command(&mut pipeline, &mut command);
                        push_pipeline(&mut block, &mut pipeline);
                    }
                    Kind::Comment => {
                        command.comments.push(span);
                        comment = None;
                    }
                    _ => command.parts.push(span),
                },
                Mode::Normal => {
                    if let Some((source, operator)) = file_redirection.take() {
                        let missing = |error: &mut Option<ParseError>, at: Span| {
                            if error.is_none() {
                                *error = Some(ParseError::Expected("redirection target", at));
                            }
                        };
                        match kind {
                            Kind::PipePipe => {
                                error.get_or_insert(ParseError::ShellOrOr(span));
                                command.parts.extend([operator, span]);
                            }
                            Kind::Item => {
                                let target = LiteTarget::File {
                                    connector: operator,
                                };
                                if let Err(refused) = command.add_redirection(source, target) {
                                    error.get_or_insert(refused);
                                    command.parts.extend([operator, span]);
                                }
                            }
                            Kind::Assignment | Kind::File { .. } => {
                                missing(&mut error, span);
                                command.parts.extend([operator, span]);
                            }
                            Kind::Pipe | Kind::PipeRedirection(_) => {
                                missing(&mut error, span);
                                command.parts.push(operator);
                                command.pipe = Some(span);
                                push_command(&mut pipeline, &mut command);
                            }
                            Kind::Eol => {
                                missing(&mut error, span);
                                command.parts.push(operator);
                                push_command(&mut pipeline, &mut command);
                            }
                            Kind::Semicolon => {
                                missing(&mut error, span);
                                command.parts.push(operator);
                                push_command(&mut pipeline, &mut command);
                                push_pipeline(&mut block, &mut pipeline);
                            }
                            Kind::Comment => {
                                missing(&mut error, operator);
                                command.parts.push(operator);
                                command.comments.push(span);
                                comment = None;
                            }
                        }
                    } else {
                        match kind {
                            Kind::PipePipe => {
                                error.get_or_insert(ParseError::ShellOrOr(span));
                                command.parts.push(span);
                            }
                            Kind::Item => {
                                let starts_attribute =
                                    self.working_set.get_span_contents(span).starts_with(b"@");
                                if starts_attribute {
                                    if let Kind::Eol | Kind::Semicolon = last {
                                        mode = Mode::Attribute;
                                    }
                                } else if let Some(comment) = comment.take() {
                                    command.comments = comment;
                                }
                                command.parts.push(span);
                            }
                            Kind::Assignment => {
                                mode = Mode::Assignment;
                                if let Some(comment) = comment.take() {
                                    command.comments = comment;
                                }
                                command.parts.push(span);
                            }
                            Kind::File { source, .. } => {
                                if let Some(refused) = command.accepts_redirection(span) {
                                    error.get_or_insert(refused);
                                }
                                file_redirection = Some((source, span));
                            }
                            Kind::PipeRedirection(source) => {
                                let target = LiteTarget::Pipe { connector: span };
                                if let Err(refused) = command.add_redirection(source, target) {
                                    error.get_or_insert(refused);
                                }
                                command.pipe = Some(span);
                                push_command(&mut pipeline, &mut command);
                            }
                            Kind::Pipe => {
                                command.pipe = Some(span);
                                push_command(&mut pipeline, &mut command);
                            }
                            Kind::Eol => {
                                if last_non_comment(&kinds, index) != Some(Kind::Pipe) {
                                    push_command(&mut pipeline, &mut command);
                                    push_pipeline(&mut block, &mut pipeline);
                                }
                                if last == Kind::Eol {
                                    comment = None;
                                }
                            }
                            Kind::Semicolon => {
                                push_command(&mut pipeline, &mut command);
                                push_pipeline(&mut block, &mut pipeline);
                            }
                            // A comment beside something, or one before it.
                            Kind::Comment => {
                                if last != Kind::Eol {
                                    command.comments.push(span);
                                    comment = None;
                                } else {
                                    comment.get_or_insert_with(Vec::new).push(span);
                                }
                            }
                        }
                    }
                }
            }
            last = kind;
        }
        if let Some((_, operator)) = file_redirection {
            command.parts.push(operator);
            error.get_or_insert(ParseError::Expected("redirection target", operator));
        }
        if mode == Mode::Attribute {
            command.attribute_idx.push(command.parts.len());
        }
        push_command(&mut pipeline, &mut command);
        push_pipeline(&mut block, &mut pipeline);
        if last_non_comment(&kinds, kinds.len()) == Some(Kind::Pipe) {
            let last = tokens[tokens.len() - 1].1;
            return (
                block,
                Some(ParseError::UnexpectedEof(
                    "pipeline missing end".into(),
                    last,
                )),
            );
        }
        (block, error)
    }
}
