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
    let mut curr_token = 0;

    let mut block = LiteBlock::new();
    let mut curr_pipeline = LiteStatement::new();
    let mut curr_command = LiteCommand::new();

    while let Some(token) = tokens.get(curr_token) {
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
                }
                curr_command = LiteCommand::new();

                if !curr_pipeline.is_empty() {
                    block.push(curr_pipeline);
                }
                curr_pipeline = LiteStatement::new();
            }
            TokenContents::Comment => {
                curr_command.comments.push(token.span);
            }
        }
        curr_token += 1;
    }
    if !curr_command.is_empty() {
        curr_pipeline.push(curr_command);
    }

    if !curr_pipeline.is_empty() {
        block.push(curr_pipeline);
    }

    (block, None)
}

#[cfg(test)]
mod tests {
    use crate::{lex, lite_parse, LiteBlock, ParseError, Span};

    fn lite_parse_helper(input: &[u8]) -> Result<LiteBlock, ParseError> {
        let (output, err) = lex(input, 0, &[], &[]);
        if let Some(err) = err {
            return Err(err);
        }

        let (output, err) = lite_parse(&output);
        if let Some(err) = err {
            return Err(err);
        }

        Ok(output)
    }

    #[test]
    fn comment_before() -> Result<(), ParseError> {
        let input = b"# this is a comment\ndef foo bar";

        let lite_block = lite_parse_helper(input)?;

        assert_eq!(lite_block.block.len(), 1);
        assert_eq!(lite_block.block[0].commands.len(), 1);
        assert_eq!(lite_block.block[0].commands[0].comments.len(), 1);
        assert_eq!(lite_block.block[0].commands[0].parts.len(), 3);

        Ok(())
    }

    #[test]
    fn comment_beside() -> Result<(), ParseError> {
        let input = b"def foo bar # this is a comment";

        let lite_block = lite_parse_helper(input)?;

        assert_eq!(lite_block.block.len(), 1);
        assert_eq!(lite_block.block[0].commands.len(), 1);
        assert_eq!(lite_block.block[0].commands[0].comments.len(), 1);
        assert_eq!(lite_block.block[0].commands[0].parts.len(), 3);

        Ok(())
    }

    #[test]
    fn comments_stack() -> Result<(), ParseError> {
        let input = b"# this is a comment\n# another comment\ndef foo bar ";

        let lite_block = lite_parse_helper(input)?;

        assert_eq!(lite_block.block.len(), 1);
        assert_eq!(lite_block.block[0].commands.len(), 1);
        assert_eq!(lite_block.block[0].commands[0].comments.len(), 2);
        assert_eq!(lite_block.block[0].commands[0].parts.len(), 3);

        Ok(())
    }

    #[test]
    fn separated_comments_dont_stack() -> Result<(), ParseError> {
        let input = b"# this is a comment\n\n# another comment\ndef foo bar ";

        let lite_block = lite_parse_helper(input)?;

        assert_eq!(lite_block.block.len(), 1);
        assert_eq!(lite_block.block[0].commands.len(), 1);
        assert_eq!(lite_block.block[0].commands[0].comments.len(), 1);
        assert_eq!(
            lite_block.block[0].commands[0].comments[0],
            Span { start: 21, end: 39 }
        );
        assert_eq!(lite_block.block[0].commands[0].parts.len(), 3);

        Ok(())
    }
}
