use std::iter::Peekable;
use std::str::CharIndices;

use crate::errors::ParseError;
use nu_source::{Span, Spanned, SpannedItem};

type Input<'t> = Peekable<CharIndices<'t>>;

#[derive(Debug, Clone)]
pub struct LiteCommand {
    pub name: Spanned<String>,
    pub args: Vec<Spanned<String>>,
}

impl LiteCommand {
    fn new(name: Spanned<String>) -> LiteCommand {
        LiteCommand { name, args: vec![] }
    }
}

#[derive(Debug)]
pub struct LitePipeline {
    pub commands: Vec<LiteCommand>,
}

fn skip_whitespace(src: &mut Input) {
    while let Some((_, x)) = src.peek() {
        if x.is_whitespace() {
            let _ = src.next();
        } else {
            break;
        }
    }
}

fn bare(src: &mut Input, span_offset: usize) -> Result<Spanned<String>, ParseError> {
    skip_whitespace(src);

    let mut bare = String::new();
    let start_offset = if let Some((pos, _)) = src.peek() {
        *pos
    } else {
        0
    };

    let mut delimiter = ' ';
    let mut inside_quote = false;
    let mut block_level = vec![];

    for (_, c) in src {
        if inside_quote {
            if c == delimiter {
                inside_quote = false;
            }
        } else if c == '\'' || c == '"' {
            inside_quote = true;
            delimiter = c;
        } else if c == '[' {
            block_level.push(c);
        } else if c == ']' {
            if let Some('[') = block_level.last() {
                let _ = block_level.pop();
            }
        } else if c == '{' {
            block_level.push(c);
        } else if c == '}' {
            if let Some('{') = block_level.last() {
                let _ = block_level.pop();
            }
        } else if block_level.is_empty() && c.is_whitespace() {
            break;
        }
        bare.push(c);
    }

    let span = Span::new(
        start_offset + span_offset,
        start_offset + span_offset + bare.len(),
    );
    Ok(bare.spanned(span))
}

fn quoted(
    src: &mut Input,
    delimiter: char,
    span_offset: usize,
) -> Result<Spanned<String>, ParseError> {
    skip_whitespace(src);

    let mut quoted_string = String::new();
    let start_offset = if let Some((pos, _)) = src.peek() {
        *pos
    } else {
        0
    };

    let _ = src.next();

    for (_, c) in src {
        if c != delimiter {
            quoted_string.push(c);
        } else {
            break;
        }
    }

    quoted_string.insert(0, delimiter);
    quoted_string.push(delimiter);

    let span = Span::new(
        start_offset + span_offset,
        start_offset + span_offset + quoted_string.len(),
    );
    Ok(quoted_string.spanned(span))
}

fn command(src: &mut Input, span_offset: usize) -> Result<LiteCommand, ParseError> {
    let command = bare(src, span_offset)?;

    if command.item.is_empty() {
        Err(ParseError::UnexpectedEndOfLine(command.span))
    } else {
        Ok(LiteCommand::new(command))
    }
}

fn pipeline(src: &mut Input, span_offset: usize) -> Result<LitePipeline, ParseError> {
    let mut commands = vec![];

    skip_whitespace(src);

    while src.peek().is_some() {
        // If there is content there, let's parse it

        let mut cmd = match command(src, span_offset) {
            Ok(cmd) => cmd,
            Err(e) => return Err(e),
        };

        skip_whitespace(src);
        while let Some((_, c)) = src.peek() {
            // The first character tells us a lot about each argument
            match c {
                '|' => {
                    // this is the end of this command
                    let _ = src.next();
                    break;
                }
                '"' | '\'' => {
                    let c = *c;
                    // quoted string
                    let arg = quoted(src, c, span_offset)?;
                    cmd.args.push(arg);
                }
                _ => {
                    // basic argument
                    let arg = bare(src, span_offset)?;
                    cmd.args.push(arg);
                }
            }
        }
        commands.push(cmd);
        skip_whitespace(src);
    }

    Ok(LitePipeline { commands })
}

pub fn lite_parse(src: &str, span_offset: usize) -> Result<LitePipeline, ParseError> {
    pipeline(&mut src.char_indices().peekable(), span_offset)
}

#[test]
fn lite_simple_1() -> Result<(), ParseError> {
    let result = lite_parse("foo", 0)?;
    assert_eq!(result.commands.len(), 1);
    assert_eq!(result.commands[0].name.span.start(), 0);
    assert_eq!(result.commands[0].name.span.end(), 3);

    Ok(())
}

#[test]
fn lite_simple_offset() -> Result<(), ParseError> {
    let result = lite_parse("foo", 10)?;
    assert_eq!(result.commands.len(), 1);
    assert_eq!(result.commands[0].name.span.start(), 10);
    assert_eq!(result.commands[0].name.span.end(), 13);

    Ok(())
}
