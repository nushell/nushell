//! Lite parsing converts a flat stream of tokens from the lexer to a syntax element structure that
//! can be parsed.

use crate::{Token, TokenContents};
use nu_protocol::{ast::RedirectionSource, ParseError, Span};
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

#[derive(Debug, Clone, Default)]
pub struct LiteCommand {
    pub pipe: Option<Span>,
    pub comments: Vec<Span>,
    pub parts: Vec<Span>,
    pub redirection: Option<LiteRedirection>,
}

impl LiteCommand {
    fn push(&mut self, span: Span) {
        self.parts.push(span);
    }

    fn try_add_redirection(
        &mut self,
        source: RedirectionSource,
        target: LiteRedirectionTarget,
    ) -> Result<(), ParseError> {
        let redirection = match (self.redirection.take(), source) {
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

pub fn lite_parse(tokens: &[Token]) -> (LiteBlock, Option<ParseError>) {
    if tokens.is_empty() {
        return (LiteBlock::default(), None);
    }

    let mut block = LiteBlock::default();
    let mut pipeline = LitePipeline::default();
    let mut command = LiteCommand::default();

    let mut last_token = TokenContents::Eol;
    let mut file_redirection = None;
    let mut curr_comment: Option<Vec<Span>> = None;
    let mut error = None;

    for (idx, token) in tokens.iter().enumerate() {
        if let Some((source, append, span)) = file_redirection.take() {
            if command.parts.is_empty() {
                error = error.or(Some(ParseError::LabeledError(
                    "Redirection without command or expression".into(),
                    "there is nothing to redirect".into(),
                    span,
                )));

                command.push(span);

                match token.contents {
                    TokenContents::Comment => {
                        command.comments.push(token.span);
                        curr_comment = None;
                    }
                    TokenContents::Pipe
                    | TokenContents::ErrGreaterPipe
                    | TokenContents::OutErrGreaterPipe => {
                        pipeline.push(&mut command);
                        command.pipe = Some(token.span);
                    }
                    TokenContents::Semicolon => {
                        pipeline.push(&mut command);
                        block.push(&mut pipeline);
                    }
                    TokenContents::Eol => {
                        pipeline.push(&mut command);
                    }
                    _ => command.push(token.span),
                }
            } else {
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
                    TokenContents::OutGreaterThan
                    | TokenContents::OutGreaterGreaterThan
                    | TokenContents::ErrGreaterThan
                    | TokenContents::ErrGreaterGreaterThan
                    | TokenContents::OutErrGreaterThan
                    | TokenContents::OutErrGreaterGreaterThan => {
                        error =
                            error.or(Some(ParseError::Expected("redirection target", token.span)));
                        command.push(span);
                        command.push(token.span);
                    }
                    TokenContents::Pipe
                    | TokenContents::ErrGreaterPipe
                    | TokenContents::OutErrGreaterPipe => {
                        error =
                            error.or(Some(ParseError::Expected("redirection target", token.span)));
                        command.push(span);
                        pipeline.push(&mut command);
                        command.pipe = Some(token.span);
                    }
                    TokenContents::Eol => {
                        error =
                            error.or(Some(ParseError::Expected("redirection target", token.span)));
                        command.push(span);
                        pipeline.push(&mut command);
                    }
                    TokenContents::Semicolon => {
                        error =
                            error.or(Some(ParseError::Expected("redirection target", token.span)));
                        command.push(span);
                        pipeline.push(&mut command);
                        block.push(&mut pipeline);
                    }
                    TokenContents::Comment => {
                        error = error.or(Some(ParseError::Expected("redirection target", span)));
                        command.push(span);
                        command.comments.push(token.span);
                        curr_comment = None;
                    }
                }
            }
        } else {
            match &token.contents {
                TokenContents::PipePipe => {
                    error = error.or(Some(ParseError::ShellOrOr(token.span)));
                    command.push(token.span);
                }
                TokenContents::Item => {
                    // This is commented out to preserve old parser behavior,
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

                    // If we have a comment, go ahead and attach it
                    if let Some(curr_comment) = curr_comment.take() {
                        command.comments = curr_comment;
                    }
                    command.push(token.span);
                }
                TokenContents::OutGreaterThan => {
                    file_redirection = Some((RedirectionSource::Stdout, false, token.span));
                }
                TokenContents::OutGreaterGreaterThan => {
                    file_redirection = Some((RedirectionSource::Stdout, true, token.span));
                }
                TokenContents::ErrGreaterThan => {
                    file_redirection = Some((RedirectionSource::Stderr, false, token.span));
                }
                TokenContents::ErrGreaterGreaterThan => {
                    file_redirection = Some((RedirectionSource::Stderr, true, token.span));
                }
                TokenContents::OutErrGreaterThan => {
                    file_redirection =
                        Some((RedirectionSource::StdoutAndStderr, false, token.span));
                }
                TokenContents::OutErrGreaterGreaterThan => {
                    file_redirection = Some((RedirectionSource::StdoutAndStderr, true, token.span));
                }
                TokenContents::ErrGreaterPipe => {
                    let target = LiteRedirectionTarget::Pipe {
                        connector: token.span,
                    };
                    if let Err(err) = command.try_add_redirection(RedirectionSource::Stderr, target)
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
                    if let Err(err) =
                        command.try_add_redirection(RedirectionSource::StdoutAndStderr, target)
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

        last_token = token.contents;
    }

    if let Some((_, _, span)) = file_redirection {
        command.push(span);
        error = error.or(Some(ParseError::Expected("redirection target", span)));
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
