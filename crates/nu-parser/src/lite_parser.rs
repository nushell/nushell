/// Lite parsing converts a flat stream of tokens from the lexer to a syntax element structure that
/// can be parsed.
use crate::{Token, TokenContents};

use nu_protocol::{ast::Redirection, ParseError, Span};

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
    // final field indicates if it's in append mode
    Redirection(Span, Redirection, LiteCommand, bool),
    // SeparateRedirection variant can only be generated by two different Redirection variant
    // final bool field indicates if it's in append mode
    SeparateRedirection {
        out: (Span, LiteCommand, bool),
        err: (Span, LiteCommand, bool),
    },
    // SameTargetRedirection variant can only be generated by Command with Redirection::OutAndErr
    // redirection's final bool field indicates if it's in append mode
    SameTargetRedirection {
        cmd: (Option<Span>, LiteCommand),
        redirection: (Span, LiteCommand, bool),
    },
}

#[derive(Debug, Default)]
pub struct LitePipeline {
    pub commands: Vec<LiteElement>,
}

impl LitePipeline {
    pub fn new() -> Self {
        Self { commands: vec![] }
    }

    pub fn push(&mut self, element: LiteElement) {
        self.commands.push(element);
    }

    pub fn insert(&mut self, index: usize, element: LiteElement) {
        self.commands.insert(index, element);
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

    pub fn push(&mut self, mut pipeline: LitePipeline) {
        // once we push `pipeline` to our block
        // the block takes ownership of `pipeline`, which means that
        // our `pipeline` is complete on collecting commands.
        self.merge_redirections(&mut pipeline);
        self.merge_cmd_with_outerr_redirection(&mut pipeline);

        self.block.push(pipeline);
    }

    pub fn is_empty(&self) -> bool {
        self.block.is_empty()
    }

    fn merge_cmd_with_outerr_redirection(&self, pipeline: &mut LitePipeline) {
        let mut cmd_index = None;
        let mut outerr_index = None;
        for (index, cmd) in pipeline.commands.iter().enumerate() {
            if let LiteElement::Command(..) = cmd {
                cmd_index = Some(index);
            }
            if let LiteElement::Redirection(
                _span,
                Redirection::StdoutAndStderr,
                _target_cmd,
                _is_append_mode,
            ) = cmd
            {
                outerr_index = Some(index);
                break;
            }
        }
        if let (Some(cmd_index), Some(outerr_index)) = (cmd_index, outerr_index) {
            // we can make sure that cmd_index is less than outerr_index.
            let outerr_redirect = pipeline.commands.remove(outerr_index);
            let cmd = pipeline.commands.remove(cmd_index);
            // `outerr_redirect` and `cmd` should always be `LiteElement::Command` and `LiteElement::Redirection`
            if let (
                LiteElement::Command(cmd_span, lite_cmd),
                LiteElement::Redirection(span, _, outerr_cmd, is_append_mode),
            ) = (cmd, outerr_redirect)
            {
                pipeline.insert(
                    cmd_index,
                    LiteElement::SameTargetRedirection {
                        cmd: (cmd_span, lite_cmd),
                        redirection: (span, outerr_cmd, is_append_mode),
                    },
                )
            }
        }
    }

    fn merge_redirections(&self, pipeline: &mut LitePipeline) {
        // In case our command may contains both stdout and stderr redirection.
        // We pick them out and Combine them into one LiteElement::SeparateRedirection variant.
        let mut stdout_index = None;
        let mut stderr_index = None;
        for (index, cmd) in pipeline.commands.iter().enumerate() {
            if let LiteElement::Redirection(_span, redirection, _target_cmd, _is_append_mode) = cmd
            {
                match *redirection {
                    Redirection::Stderr => stderr_index = Some(index),
                    Redirection::Stdout => stdout_index = Some(index),
                    Redirection::StdoutAndStderr => {}
                }
            }
        }

        if let (Some(out_indx), Some(err_indx)) = (stdout_index, stderr_index) {
            let (out_redirect, err_redirect, new_indx) = {
                // to avoid panic, we need to remove commands which have larger index first.
                if out_indx > err_indx {
                    let out_redirect = pipeline.commands.remove(out_indx);
                    let err_redirect = pipeline.commands.remove(err_indx);
                    (out_redirect, err_redirect, err_indx)
                } else {
                    let err_redirect = pipeline.commands.remove(err_indx);
                    let out_redirect = pipeline.commands.remove(out_indx);
                    (out_redirect, err_redirect, out_indx)
                }
            };
            // `out_redirect` and `err_redirect` should always be `LiteElement::Redirection`
            if let (
                LiteElement::Redirection(out_span, _, out_command, out_append_mode),
                LiteElement::Redirection(err_span, _, err_command, err_append_mode),
            ) = (out_redirect, err_redirect)
            {
                // using insert with specific index to keep original
                // pipeline commands order.
                pipeline.insert(
                    new_indx,
                    LiteElement::SeparateRedirection {
                        out: (out_span, out_command, out_append_mode),
                        err: (err_span, err_command, err_append_mode),
                    },
                )
            }
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

    for (idx, token) in tokens.iter().enumerate() {
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
            | TokenContents::OutGreaterGreaterThan
            | TokenContents::ErrGreaterThan
            | TokenContents::ErrGreaterGreaterThan
            | TokenContents::OutErrGreaterThan
            | TokenContents::OutErrGreaterGreaterThan => {
                if let Some(err) = push_command_to(
                    &mut curr_pipeline,
                    curr_command,
                    last_connector,
                    last_connector_span,
                ) {
                    error = Some(err);
                }

                curr_command = LiteCommand::new();
                last_token = token.contents;
                last_connector = token.contents;
                last_connector_span = Some(token.span);
            }
            TokenContents::Pipe => {
                if let Some(err) = push_command_to(
                    &mut curr_pipeline,
                    curr_command,
                    last_connector,
                    last_connector_span,
                ) {
                    error = Some(err);
                }

                curr_command = LiteCommand::new();
                last_token = TokenContents::Pipe;
                last_connector = TokenContents::Pipe;
                last_connector_span = Some(token.span);
            }
            TokenContents::Eol => {
                // Handle `[Command] [Pipe] ([Comment] | [Eol])+ [Command]`
                //
                // `[Eol]` branch checks if previous token is `[Pipe]` to construct pipeline
                // and so `[Comment] | [Eol]` should be ignore to make it work
                let actual_token = last_non_comment_token(tokens, idx);
                if actual_token != Some(TokenContents::Pipe)
                    && actual_token != Some(TokenContents::OutGreaterThan)
                {
                    if let Some(err) = push_command_to(
                        &mut curr_pipeline,
                        curr_command,
                        last_connector,
                        last_connector_span,
                    ) {
                        error = Some(err);
                    }

                    curr_command = LiteCommand::new();
                    if !curr_pipeline.is_empty() {
                        block.push(curr_pipeline);

                        curr_pipeline = LitePipeline::new();
                        last_connector = TokenContents::Pipe;
                        last_connector_span = None;
                    }
                }

                if last_token == TokenContents::Eol {
                    // Clear out the comment as we're entering a new comment
                    curr_comment = None;
                }

                last_token = TokenContents::Eol;
            }
            TokenContents::Semicolon => {
                if let Some(err) = push_command_to(
                    &mut curr_pipeline,
                    curr_command,
                    last_connector,
                    last_connector_span,
                ) {
                    error = Some(err);
                }

                curr_command = LiteCommand::new();
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

    if let Some(err) = push_command_to(
        &mut curr_pipeline,
        curr_command,
        last_connector,
        last_connector_span,
    ) {
        error = Some(err);
    }
    if !curr_pipeline.is_empty() {
        block.push(curr_pipeline);
    }

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

fn get_redirection(connector: TokenContents) -> Option<(Redirection, bool)> {
    match connector {
        TokenContents::OutGreaterThan => Some((Redirection::Stdout, false)),
        TokenContents::OutGreaterGreaterThan => Some((Redirection::Stdout, true)),
        TokenContents::ErrGreaterThan => Some((Redirection::Stderr, false)),
        TokenContents::ErrGreaterGreaterThan => Some((Redirection::Stderr, true)),
        TokenContents::OutErrGreaterThan => Some((Redirection::StdoutAndStderr, false)),
        TokenContents::OutErrGreaterGreaterThan => Some((Redirection::StdoutAndStderr, true)),
        _ => None,
    }
}

/// push a `command` to `pipeline`
///
/// It will return Some(err) if `command` is empty and we want to push a
/// redirection command.
fn push_command_to(
    pipeline: &mut LitePipeline,
    command: LiteCommand,
    last_connector: TokenContents,
    last_connector_span: Option<Span>,
) -> Option<ParseError> {
    if !command.is_empty() {
        match get_redirection(last_connector) {
            Some((redirect, is_append_mode)) => pipeline.push(LiteElement::Redirection(
                last_connector_span.expect("internal error: redirection missing span information"),
                redirect,
                command,
                is_append_mode,
            )),
            None => pipeline.push(LiteElement::Command(last_connector_span, command)),
        }
        None
    } else if let Some(_) = get_redirection(last_connector) {
        Some(ParseError::Expected(
            "redirection target",
            last_connector_span.expect("internal error: redirection missing span information"),
        ))
    } else {
        None
    }
}
