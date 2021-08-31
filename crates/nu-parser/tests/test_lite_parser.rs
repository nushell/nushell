use nu_parser::{lex, lite_parse, LiteBlock, ParseError, Span};

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
    // Code:
    // # this is a comment
    // def foo bar
    let input = b"# this is a comment\ndef foo bar";

    let lite_block = lite_parse_helper(input)?;

    assert_eq!(lite_block.block.len(), 2);
    assert_eq!(lite_block.block[0].commands.len(), 1);
    assert_eq!(lite_block.block[0].commands[0].comments.len(), 1);
    assert_eq!(lite_block.block[1].commands[0].parts.len(), 3);

    Ok(())
}

#[test]
fn comment_beside() -> Result<(), ParseError> {
    // Code:
    // def foo bar # this is a comment
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
    // Code:
    // # this is a comment
    // # another comment
    // # def foo bar
    let input = b"# this is a comment\n# another comment\ndef foo bar ";

    let lite_block = lite_parse_helper(input)?;

    assert_eq!(lite_block.block.len(), 3);
    assert_eq!(lite_block.block[0].commands[0].comments.len(), 1);
    assert_eq!(lite_block.block[0].commands[0].parts.len(), 0);

    assert_eq!(lite_block.block[1].commands[0].comments.len(), 1);
    assert_eq!(lite_block.block[1].commands[0].parts.len(), 0);

    assert_eq!(lite_block.block[2].commands[0].comments.len(), 0);
    assert_eq!(lite_block.block[2].commands[0].parts.len(), 3);

    Ok(())
}

#[test]
fn separated_comments_dont_stack() -> Result<(), ParseError> {
    // Code:
    // # this is a comment
    //
    // # another comment
    // # def foo bar
    let input = b"# this is a comment\n\n# another comment\ndef foo bar ";

    let lite_block = lite_parse_helper(input)?;

    assert_eq!(lite_block.block.len(), 3);
    assert_eq!(lite_block.block[0].commands[0].comments.len(), 1);
    assert_eq!(lite_block.block[0].commands[0].parts.len(), 0);

    assert_eq!(lite_block.block[1].commands[0].comments.len(), 1);
    assert_eq!(lite_block.block[1].commands[0].parts.len(), 0);

    assert_eq!(lite_block.block[2].commands[0].comments.len(), 0);
    assert_eq!(lite_block.block[2].commands[0].parts.len(), 3);

    assert_eq!(
        lite_block.block[0].commands[0].comments[0],
        Span { start: 0, end: 19 }
    );
    assert_eq!(
        lite_block.block[1].commands[0].comments[0],
        Span { start: 21, end: 38 }
    );

    Ok(())
}

#[test]
fn multiple_statements() -> Result<(), ParseError> {
    // Code:
    // # A comment
    // let a = ( 3 + (
    // 4 +
    // 5 ))
    // let b = 1 # comment
    let input = b"# comment \n let a = ( 3 + (\n 4 + \n 5 )) \n let b = 1 # comment";

    let lite_block = lite_parse_helper(input)?;

    assert_eq!(lite_block.block.len(), 3);
    assert_eq!(lite_block.block[0].commands[0].comments.len(), 1);
    assert_eq!(
        lite_block.block[0].commands[0].comments[0],
        Span { start: 0, end: 10 }
    );

    assert_eq!(lite_block.block[1].commands[0].comments.len(), 0);
    assert_eq!(lite_block.block[1].commands[0].parts.len(), 4);

    assert_eq!(lite_block.block[2].commands[0].comments.len(), 1);
    assert_eq!(lite_block.block[2].commands[0].parts.len(), 4);
    assert_eq!(
        lite_block.block[2].commands[0].comments[0],
        Span { start: 52, end: 61 }
    );

    Ok(())
}

#[test]
fn multiple_commands() -> Result<(), ParseError> {
    // Pipes add commands to the lite parser
    // Code:
    // let a = ls | where name == 1
    // let b = 1 # comment
    let input = b"let a = ls | where name == 1 \n let b = 1 # comment";

    let lite_block = lite_parse_helper(input)?;

    assert_eq!(lite_block.block.len(), 2);
    assert_eq!(lite_block.block[0].commands.len(), 2);
    assert_eq!(lite_block.block[1].commands.len(), 1);

    assert_eq!(
        lite_block.block[1].commands[0].comments[0],
        Span { start: 41, end: 50 }
    );

    Ok(())
}

#[test]
fn multiple_commands_with_comment() -> Result<(), ParseError> {
    // Pipes add commands to the lite parser
    // The comments are attached to the commands next to them
    // Code:
    // let a = ls | where name == 1 # comment
    // let b = 1 # comment
    //let a = ls | where name == 1 # comment \n let b = 1 # comment
    let input = b"let a = ls | where name == 1 # comment\n let b = 1 # comment";

    let lite_block = lite_parse_helper(input)?;

    assert_eq!(lite_block.block.len(), 2);
    assert_eq!(lite_block.block[0].commands.len(), 2);
    assert_eq!(lite_block.block[1].commands.len(), 1);

    assert_eq!(
        lite_block.block[0].commands[1].comments[0],
        Span { start: 29, end: 38 }
    );

    Ok(())
}
