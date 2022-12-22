/// Lite parsing converts a flat stream of tokens from the lexer to a syntax element structure that
/// can be parsed.
use crate::{ParseError, Token, TokenContents};

use nu_protocol::{ast::Redirection, Span};

#[derive(Debug)]
pub struct LiteCommand {
    pub comments: Vec<Span>,
    pub parts: Vec<Span>,
}

impl Default for LiteCommand {
    fn default() -> Self {
        Self::new()
    }
}

impl LiteCommand {
    pub fn new() -> Self {
        Self {
            comments: vec![],
            parts: vec![],
        }
    }

    pub fn push(&mut self, span: Span) {
        self.parts.push(span);
    }

    pub fn is_empty(&self) -> bool {
        self.parts.is_empty()
    }
}

// Note: the Span is the span of the connector not the whole element
#[derive(Debug)]
pub enum LiteElement {
    Command(Option<Span>, LiteCommand),
    Redirection(Span, Redirection, LiteCommand),
}

#[derive(Debug)]
pub struct LitePipeline {
    pub commands: Vec<LiteElement>,
}

impl Default for LitePipeline {
    fn default() -> Self {
        Self::new()
    }
}

impl LitePipeline {
    pub fn new() -> Self {
        Self { commands: vec![] }
    }

    pub fn push(&mut self, element: LiteElement) {
        self.commands.push(element);
    }

    pub fn is_empty(&self) -> bool {
        self.commands.is_empty()
    }
}

#[derive(Debug)]
pub struct LiteBlock {
    pub block: Vec<LitePipeline>,
}

impl Default for LiteBlock {
    fn default() -> Self {
        Self::new()
    }
}

impl LiteBlock {
    pub fn new() -> Self {
        Self { block: vec![] }
    }

    pub fn push(&mut self, pipeline: LitePipeline) {
        self.block.push(pipeline);
    }

    pub fn is_empty(&self) -> bool {
        self.block.is_empty()
    }
}

pub fn lite_parse(tokens: &[Token]) -> (LiteBlock, Option<ParseError>) {
    let mut block = LiteBlock::new();
    let mut curr_pipeline = LitePipeline::new();
    let mut curr_command = LiteCommand::new();

    let mut last_token = TokenContents::Eol;

    let mut last_connector = TokenContents::Pipe;
    let mut last_connector_span: Option<Span> = None;

    if tokens.is_empty() {
        return (LiteBlock::new(), None);
    }

    let mut curr_comment: Option<Vec<Span>> = None;

    let mut error = None;

    for token in tokens.iter() {
        match &token.contents {
            TokenContents::PipePipe => {
                error = error.or(Some(ParseError::ShellOrOr(token.span)));
                curr_command.push(token.span);
                last_token = TokenContents::Item;
            }
            TokenContents::Item => {
                // If we have a comment, go ahead and attach it
                if let Some(curr_comment) = curr_comment.take() {
                    curr_command.comments = curr_comment;
                }
                curr_command.push(token.span);
                last_token = TokenContents::Item;
            }
            TokenContents::OutGreaterThan
            | TokenContents::ErrGreaterThan
            | TokenContents::OutErrGreaterThan => {
                if !curr_command.is_empty() {
                    match last_connector {
                        TokenContents::OutGreaterThan => {
                            curr_pipeline.push(LiteElement::Redirection(
                                token.span,
                                Redirection::Stdout,
                                curr_command,
                            ));
                        }
                        TokenContents::ErrGreaterThan => {
                            curr_pipeline.push(LiteElement::Redirection(
                                token.span,
                                Redirection::Stderr,
                                curr_command,
                            ));
                        }
                        TokenContents::OutErrGreaterThan => {
                            curr_pipeline.push(LiteElement::Redirection(
                                token.span,
                                Redirection::StdoutAndStderr,
                                curr_command,
                            ));
                        }
                        _ => {
                            curr_pipeline
                                .push(LiteElement::Command(last_connector_span, curr_command));
                        }
                    }
                    curr_command = LiteCommand::new();
                }
                last_token = token.contents;
                last_connector = token.contents;
                last_connector_span = Some(token.span);
            }
            TokenContents::Pipe => {
                if !curr_command.is_empty() {
                    match last_connector {
                        TokenContents::OutGreaterThan => {
                            curr_pipeline.push(LiteElement::Redirection(
                                token.span,
                                Redirection::Stdout,
                                curr_command,
                            ));
                        }
                        TokenContents::ErrGreaterThan => {
                            curr_pipeline.push(LiteElement::Redirection(
                                token.span,
                                Redirection::Stderr,
                                curr_command,
                            ));
                        }
                        TokenContents::OutErrGreaterThan => {
                            curr_pipeline.push(LiteElement::Redirection(
                                token.span,
                                Redirection::StdoutAndStderr,
                                curr_command,
                            ));
                        }
                        _ => {
                            curr_pipeline
                                .push(LiteElement::Command(last_connector_span, curr_command));
                        }
                    }
                    curr_command = LiteCommand::new();
                }
                last_token = TokenContents::Pipe;
                last_connector = TokenContents::Pipe;
                last_connector_span = Some(token.span);
            }
            TokenContents::Eol => {
                if last_token != TokenContents::Pipe && last_token != TokenContents::OutGreaterThan
                {
                    if !curr_command.is_empty() {
                        match last_connector {
                            TokenContents::OutGreaterThan => {
                                curr_pipeline.push(LiteElement::Redirection(
                                    last_connector_span.expect(
                                        "internal error: redirection missing span information",
                                    ),
                                    Redirection::Stdout,
                                    curr_command,
                                ));
                            }
                            TokenContents::ErrGreaterThan => {
                                curr_pipeline.push(LiteElement::Redirection(
                                    last_connector_span.expect(
                                        "internal error: redirection missing span information",
                                    ),
                                    Redirection::Stderr,
                                    curr_command,
                                ));
                            }
                            TokenContents::OutErrGreaterThan => {
                                curr_pipeline.push(LiteElement::Redirection(
                                    last_connector_span.expect(
                                        "internal error: redirection missing span information",
                                    ),
                                    Redirection::StdoutAndStderr,
                                    curr_command,
                                ));
                            }
                            _ => {
                                curr_pipeline
                                    .push(LiteElement::Command(last_connector_span, curr_command));
                            }
                        }

                        curr_command = LiteCommand::new();
                    }

                    if !curr_pipeline.is_empty() {
                        block.push(curr_pipeline);

                        curr_pipeline = LitePipeline::new();
                    }
                }

                if last_token == TokenContents::Eol {
                    // Clear out the comment as we're entering a new comment
                    curr_comment = None;
                }

                last_token = TokenContents::Eol;
            }
            TokenContents::Semicolon => {
                if !curr_command.is_empty() {
                    match last_connector {
                        TokenContents::OutGreaterThan => {
                            curr_pipeline.push(LiteElement::Redirection(
                                last_connector_span
                                    .expect("internal error: redirection missing span information"),
                                Redirection::Stdout,
                                curr_command,
                            ));
                        }
                        TokenContents::ErrGreaterThan => {
                            curr_pipeline.push(LiteElement::Redirection(
                                last_connector_span
                                    .expect("internal error: redirection missing span information"),
                                Redirection::Stderr,
                                curr_command,
                            ));
                        }
                        TokenContents::OutErrGreaterThan => {
                            curr_pipeline.push(LiteElement::Redirection(
                                last_connector_span
                                    .expect("internal error: redirection missing span information"),
                                Redirection::StdoutAndStderr,
                                curr_command,
                            ));
                        }
                        _ => {
                            curr_pipeline
                                .push(LiteElement::Command(last_connector_span, curr_command));
                        }
                    }

                    curr_command = LiteCommand::new();
                }

                if !curr_pipeline.is_empty() {
                    block.push(curr_pipeline);

                    curr_pipeline = LitePipeline::new();
                    last_connector = TokenContents::Pipe;
                    last_connector_span = None;
                }

                last_token = TokenContents::Semicolon;
            }
            TokenContents::Comment => {
                // Comment is beside something
                if last_token != TokenContents::Eol {
                    curr_command.comments.push(token.span);
                    curr_comment = None;
                } else {
                    // Comment precedes something
                    if let Some(curr_comment) = &mut curr_comment {
                        curr_comment.push(token.span);
                    } else {
                        curr_comment = Some(vec![token.span]);
                    }
                }

                last_token = TokenContents::Comment;
            }
        }
    }

    if !curr_command.is_empty() {
        match last_connector {
            TokenContents::OutGreaterThan => {
                curr_pipeline.push(LiteElement::Redirection(
                    last_connector_span
                        .expect("internal error: redirection missing span information"),
                    Redirection::Stdout,
                    curr_command,
                ));
            }
            TokenContents::ErrGreaterThan => {
                curr_pipeline.push(LiteElement::Redirection(
                    last_connector_span
                        .expect("internal error: redirection missing span information"),
                    Redirection::Stderr,
                    curr_command,
                ));
            }
            TokenContents::OutErrGreaterThan => {
                curr_pipeline.push(LiteElement::Redirection(
                    last_connector_span
                        .expect("internal error: redirection missing span information"),
                    Redirection::StdoutAndStderr,
                    curr_command,
                ));
            }
            _ => {
                curr_pipeline.push(LiteElement::Command(last_connector_span, curr_command));
            }
        }
    }

    if !curr_pipeline.is_empty() {
        block.push(curr_pipeline);
    }

    if last_token == TokenContents::Pipe {
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
