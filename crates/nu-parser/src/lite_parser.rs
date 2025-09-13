//! Lite parsing converts a flat stream of tokens from the lexer to a syntax element structure that
//! can be parsed.

use crate::{Token, TokenContents};
use itertools::{Either, Itertools};
use nu_protocol::{ParseError, Span, ast::RedirectionSource, engine::StateWorkingSet};
use std::mem;

#[derive(Debug, Clone, Copy)]
pub enum LiteRedirectionTarget {
    File {
        connector: Span,
        file: Span,
        append: bool,
    },
    Pipe {
        connector: Span,
    },
}

impl LiteRedirectionTarget {
    pub fn connector(&self) -> Span {
        match self {
            LiteRedirectionTarget::File { connector, .. }
            | LiteRedirectionTarget::Pipe { connector } => *connector,
        }
    }

    pub fn spans(&self) -> impl Iterator<Item = Span> {
        match *self {
            LiteRedirectionTarget::File {
                connector, file, ..
            } => Either::Left([connector, file].into_iter()),
            LiteRedirectionTarget::Pipe { connector } => Either::Right(std::iter::once(connector)),
        }
    }
}

#[derive(Debug, Clone)]
pub enum LiteRedirection {
    Single {
        source: RedirectionSource,
        target: LiteRedirectionTarget,
    },
    Separate {
        out: LiteRedirectionTarget,
        err: LiteRedirectionTarget,
    },
}

impl LiteRedirection {
    pub fn spans(&self) -> impl Iterator<Item = Span> {
        match self {
            LiteRedirection::Single { target, .. } => Either::Left(target.spans()),
            LiteRedirection::Separate { out, err } => {
                Either::Right(out.spans().chain(err.spans()).sorted())
            }
        }
    }
}

#[derive(Debug, Clone, Default)]
pub struct LiteCommand {
    pub pipe: Option<Span>,
    pub comments: Vec<Span>,
    pub parts: Vec<Span>,
    pub redirection: Option<LiteRedirection>,
    /// one past the end indices of attributes
    pub attribute_idx: Vec<usize>,
}

impl LiteCommand {
    fn push(&mut self, span: Span) {
        self.parts.push(span);
    }

    fn check_accepts_redirection(&self, span: Span) -> Option<ParseError> {
        self.parts
            .is_empty()
            .then_some(ParseError::UnexpectedRedirection { span })
    }

    fn try_add_redirection(
        &mut self,
        source: RedirectionSource,
        target: LiteRedirectionTarget,
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

    pub fn parts_including_redirection(&self) -> impl Iterator<Item = Span> + '_ {
        self.parts
            .iter()
            .copied()
            .chain(
                self.redirection
                    .iter()
                    .flat_map(|redirection| redirection.spans()),
            )
            .sorted_unstable_by_key(|a| (a.start, a.end))
    }

    pub fn command_parts(&self) -> &[Span] {
        let command_start = self.attribute_idx.last().copied().unwrap_or(0);
        &self.parts[command_start..]
    }

    pub fn has_attributes(&self) -> bool {
        !self.attribute_idx.is_empty()
    }

    pub fn attribute_commands(&'_ self) -> impl Iterator<Item = LiteCommand> + '_ {
        std::iter::once(0)
            .chain(self.attribute_idx.iter().copied())
            .tuple_windows()
            .map(|(s, e)| LiteCommand {
                parts: self.parts[s..e].to_owned(),
                ..Default::default()
            })
    }
}

#[derive(Debug, Clone, Default)]
pub struct LitePipeline {
    pub commands: Vec<LiteCommand>,
}

impl LitePipeline {
    fn push(&mut self, element: &mut LiteCommand) {
        if !element.parts.is_empty() || element.redirection.is_some() {
            self.commands.push(mem::take(element));
        }
    }
}

#[derive(Debug, Clone, Default)]
pub struct LiteBlock {
    pub block: Vec<LitePipeline>,
}

impl LiteBlock {
    fn push(&mut self, pipeline: &mut LitePipeline) {
        if !pipeline.commands.is_empty() {
            self.block.push(mem::take(pipeline));
        }
    }
}

fn last_non_comment_token(tokens: &[Token], cur_idx: usize) -> Option<TokenContents> {
    let mut expect = TokenContents::Comment;
    for token in tokens.iter().take(cur_idx).rev() {
        // skip ([Comment]+ [Eol]) pair
        match (token.contents, expect) {
            (TokenContents::Comment, TokenContents::Comment)
            | (TokenContents::Comment, TokenContents::Eol) => expect = TokenContents::Eol,
            (TokenContents::Eol, TokenContents::Eol) => expect = TokenContents::Comment,
            (token, _) => return Some(token),
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

pub fn lite_parse(
    tokens: &[Token],
    working_set: &StateWorkingSet,
) -> (LiteBlock, Option<ParseError>) {
    if tokens.is_empty() {
        return (LiteBlock::default(), None);
    }

    let mut block = LiteBlock::default();
    let mut pipeline = LitePipeline::default();
    let mut command = LiteCommand::default();

    let mut last_token = TokenContents::Eol;
    let mut file_redirection = None;
    let mut curr_comment: Option<Vec<Span>> = None;
    let mut mode = Mode::Normal;
    let mut error = None;

    for (idx, token) in tokens.iter().enumerate() {
        match mode {
            Mode::Attribute => {
                match &token.contents {
                    // Consume until semicolon or terminating EOL. Attributes can't contain pipelines or redirections.
                    TokenContents::Eol | TokenContents::Semicolon => {
                        command.attribute_idx.push(command.parts.len());
                        mode = Mode::Normal;
                        if let TokenContents::Eol | TokenContents::Semicolon = last_token {
                            // Clear out the comment as we're entering a new comment
                            curr_comment = None;
                            pipeline.push(&mut command);
                            block.push(&mut pipeline);
                        }
                    }
                    TokenContents::Comment => {
                        command.comments.push(token.span);
                        curr_comment = None;
                    }
                    _ => command.push(token.span),
                }
            }
            Mode::Assignment => {
                match &token.contents {
                    // Consume until semicolon or terminating EOL. Assignments absorb pipelines and
                    // redirections.
                    TokenContents::Eol => {
                        // Handle `[Command] [Pipe] ([Comment] | [Eol])+ [Command]`
                        //
                        // `[Eol]` branch checks if previous token is `[Pipe]` to construct pipeline
                        // and so `[Comment] | [Eol]` should be ignore to make it work
                        let actual_token = last_non_comment_token(tokens, idx);
                        if actual_token != Some(TokenContents::Pipe) {
                            mode = Mode::Normal;
                            pipeline.push(&mut command);
                            block.push(&mut pipeline);
                        }

                        if last_token == TokenContents::Eol {
                            // Clear out the comment as we're entering a new comment
                            curr_comment = None;
                        }
                    }
                    TokenContents::Semicolon => {
                        mode = Mode::Normal;
                        pipeline.push(&mut command);
                        block.push(&mut pipeline);
                    }
                    TokenContents::Comment => {
                        command.comments.push(token.span);
                        curr_comment = None;
                    }
                    _ => command.push(token.span),
                }
            }
            Mode::Normal => {
                if let Some((source, append, span)) = file_redirection.take() {
                    match &token.contents {
                        TokenContents::PipePipe => {
                            error = error.or(Some(ParseError::ShellOrOr(token.span)));
                            command.push(span);
                            command.push(token.span);
                        }
                        TokenContents::Item => {
                            let target = LiteRedirectionTarget::File {
                                connector: span,
                                file: token.span,
                                append,
                            };
                            if let Err(err) = command.try_add_redirection(source, target) {
                                error = error.or(Some(err));
                                command.push(span);
                                command.push(token.span)
                            }
                        }
                        TokenContents::AssignmentOperator => {
                            error = error
                                .or(Some(ParseError::Expected("redirection target", token.span)));
                            command.push(span);
                            command.push(token.span);
                        }
                        TokenContents::OutGreaterThan
                        | TokenContents::OutGreaterGreaterThan
                        | TokenContents::ErrGreaterThan
                        | TokenContents::ErrGreaterGreaterThan
                        | TokenContents::OutErrGreaterThan
                        | TokenContents::OutErrGreaterGreaterThan => {
                            error = error
                                .or(Some(ParseError::Expected("redirection target", token.span)));
                            command.push(span);
                            command.push(token.span);
                        }
                        TokenContents::Pipe
                        | TokenContents::ErrGreaterPipe
                        | TokenContents::OutErrGreaterPipe => {
                            error = error
                                .or(Some(ParseError::Expected("redirection target", token.span)));
                            command.push(span);
                            pipeline.push(&mut command);
                            command.pipe = Some(token.span);
                        }
                        TokenContents::Eol => {
                            error = error
                                .or(Some(ParseError::Expected("redirection target", token.span)));
                            command.push(span);
                            pipeline.push(&mut command);
                        }
                        TokenContents::Semicolon => {
                            error = error
                                .or(Some(ParseError::Expected("redirection target", token.span)));
                            command.push(span);
                            pipeline.push(&mut command);
                            block.push(&mut pipeline);
                        }
                        TokenContents::Comment => {
                            error =
                                error.or(Some(ParseError::Expected("redirection target", span)));
                            command.push(span);
                            command.comments.push(token.span);
                            curr_comment = None;
                        }
                    }
                } else {
                    match &token.contents {
                        TokenContents::PipePipe => {
                            error = error.or(Some(ParseError::ShellOrOr(token.span)));
                            command.push(token.span);
                        }
                        TokenContents::Item => {
                            // FIXME: This is commented out to preserve old parser behavior,
                            // but we should probably error here.
                            //
                            // if element.redirection.is_some() {
                            //     error = error.or(Some(ParseError::LabeledError(
                            //         "Unexpected positional".into(),
                            //         "cannot add positional arguments after output redirection".into(),
                            //         token.span,
                            //     )));
                            // }
                            //
                            // For example, this is currently allowed: ^echo thing o> out.txt extra_arg

                            if working_set.get_span_contents(token.span).starts_with(b"@") {
                                if let TokenContents::Eol | TokenContents::Semicolon = last_token {
                                    mode = Mode::Attribute;
                                }
                                command.push(token.span);
                            } else {
                                // If we have a comment, go ahead and attach it
                                if let Some(curr_comment) = curr_comment.take() {
                                    command.comments = curr_comment;
                                }
                                command.push(token.span);
                            }
                        }
                        TokenContents::AssignmentOperator => {
                            // When in assignment mode, we'll just consume pipes or redirections as part of
                            // the command.
                            mode = Mode::Assignment;
                            if let Some(curr_comment) = curr_comment.take() {
                                command.comments = curr_comment;
                            }
                            command.push(token.span);
                        }
                        TokenContents::OutGreaterThan => {
                            error = error.or(command.check_accepts_redirection(token.span));
                            file_redirection = Some((RedirectionSource::Stdout, false, token.span));
                        }
                        TokenContents::OutGreaterGreaterThan => {
                            error = error.or(command.check_accepts_redirection(token.span));
                            file_redirection = Some((RedirectionSource::Stdout, true, token.span));
                        }
                        TokenContents::ErrGreaterThan => {
                            error = error.or(command.check_accepts_redirection(token.span));
                            file_redirection = Some((RedirectionSource::Stderr, false, token.span));
                        }
                        TokenContents::ErrGreaterGreaterThan => {
                            error = error.or(command.check_accepts_redirection(token.span));
                            file_redirection = Some((RedirectionSource::Stderr, true, token.span));
                        }
                        TokenContents::OutErrGreaterThan => {
                            error = error.or(command.check_accepts_redirection(token.span));
                            file_redirection =
                                Some((RedirectionSource::StdoutAndStderr, false, token.span));
                        }
                        TokenContents::OutErrGreaterGreaterThan => {
                            error = error.or(command.check_accepts_redirection(token.span));
                            file_redirection =
                                Some((RedirectionSource::StdoutAndStderr, true, token.span));
                        }
                        TokenContents::ErrGreaterPipe => {
                            let target = LiteRedirectionTarget::Pipe {
                                connector: token.span,
                            };
                            if let Err(err) =
                                command.try_add_redirection(RedirectionSource::Stderr, target)
                            {
                                error = error.or(Some(err));
                            }
                            pipeline.push(&mut command);
                            command.pipe = Some(token.span);
                        }
                        TokenContents::OutErrGreaterPipe => {
                            let target = LiteRedirectionTarget::Pipe {
                                connector: token.span,
                            };
                            if let Err(err) = command
                                .try_add_redirection(RedirectionSource::StdoutAndStderr, target)
                            {
                                error = error.or(Some(err));
                            }
                            pipeline.push(&mut command);
                            command.pipe = Some(token.span);
                        }
                        TokenContents::Pipe => {
                            pipeline.push(&mut command);
                            command.pipe = Some(token.span);
                        }
                        TokenContents::Eol => {
                            // Handle `[Command] [Pipe] ([Comment] | [Eol])+ [Command]`
                            //
                            // `[Eol]` branch checks if previous token is `[Pipe]` to construct pipeline
                            // and so `[Comment] | [Eol]` should be ignore to make it work
                            let actual_token = last_non_comment_token(tokens, idx);
                            if actual_token != Some(TokenContents::Pipe) {
                                pipeline.push(&mut command);
                                block.push(&mut pipeline);
                            }

                            if last_token == TokenContents::Eol {
                                // Clear out the comment as we're entering a new comment
                                curr_comment = None;
                            }
                        }
                        TokenContents::Semicolon => {
                            pipeline.push(&mut command);
                            block.push(&mut pipeline);
                        }
                        TokenContents::Comment => {
                            // Comment is beside something
                            if last_token != TokenContents::Eol {
                                command.comments.push(token.span);
                                curr_comment = None;
                            } else {
                                // Comment precedes something
                                if let Some(curr_comment) = &mut curr_comment {
                                    curr_comment.push(token.span);
                                } else {
                                    curr_comment = Some(vec![token.span]);
                                }
                            }
                        }
                    }
                }
            }
        }

        last_token = token.contents;
    }

    if let Some((_, _, span)) = file_redirection {
        command.push(span);
        error = error.or(Some(ParseError::Expected("redirection target", span)));
    }

    if let Mode::Attribute = mode {
        command.attribute_idx.push(command.parts.len());
    }

    pipeline.push(&mut command);
    block.push(&mut pipeline);

    if last_non_comment_token(tokens, tokens.len()) == Some(TokenContents::Pipe) {
        (
            block,
            Some(ParseError::UnexpectedEof(
                "pipeline missing end".into(),
                tokens[tokens.len() - 1].span,
            )),
        )
    } else {
        (block, error)
    }
}
