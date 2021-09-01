use crate::{ParseError, Span, Token, TokenContents};

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

    pub fn is_empty_comments(&self) -> bool {
        self.comments.is_empty()
    }
}

#[derive(Debug)]
pub struct LiteStatement {
    pub commands: Vec<LiteCommand>,
}

impl Default for LiteStatement {
    fn default() -> Self {
        Self::new()
    }
}

impl LiteStatement {
    pub fn new() -> Self {
        Self { commands: vec![] }
    }

    pub fn push(&mut self, command: LiteCommand) {
        self.commands.push(command);
    }

    pub fn is_empty(&self) -> bool {
        self.commands.is_empty()
    }
}

#[derive(Debug)]
pub struct LiteBlock {
    pub block: Vec<LiteStatement>,
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

    pub fn push(&mut self, pipeline: LiteStatement) {
        self.block.push(pipeline);
    }

    pub fn is_empty(&self) -> bool {
        self.block.is_empty()
    }
}

pub fn lite_parse(tokens: &[Token]) -> (LiteBlock, Option<ParseError>) {
    let mut block = LiteBlock::new();
    let mut curr_pipeline = LiteStatement::new();
    let mut curr_command = LiteCommand::new();

    for token in tokens.iter() {
        match &token.contents {
            TokenContents::Item => curr_command.push(token.span),
            TokenContents::Pipe => {
                if !curr_command.is_empty() {
                    curr_pipeline.push(curr_command);
                    curr_command = LiteCommand::new();
                }
            }
            TokenContents::Eol | TokenContents::Semicolon => {
                if !curr_command.is_empty() {
                    curr_pipeline.push(curr_command);

                    curr_command = LiteCommand::new();
                }

                if !curr_pipeline.is_empty() {
                    block.push(curr_pipeline);

                    curr_pipeline = LiteStatement::new();
                }
            }
            TokenContents::Comment => {
                curr_command.comments.push(token.span);
            }
        }
    }

    if !curr_command.is_empty() {
        curr_pipeline.push(curr_command);
    }

    if !curr_pipeline.is_empty() {
        block.push(curr_pipeline);
    }

    (block, None)
}
