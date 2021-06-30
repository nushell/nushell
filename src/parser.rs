use std::str::Utf8Error;

use crate::{lex, lite_parse, LiteBlock, LiteStatement, ParseError, ParserWorkingSet, Span};

#[derive(Debug)]
pub enum Expression {}

#[derive(Debug)]
pub enum Import {}

#[derive(Debug)]
pub struct Block {
    stmts: Vec<Statement>,
}

impl Block {
    pub fn new() -> Self {
        Self { stmts: vec![] }
    }
}

#[derive(Debug)]
pub struct VarDecl {
    name: String,
    value: Expression,
}

#[derive(Debug)]
pub enum Statement {
    Pipeline(Pipeline),
    VarDecl(VarDecl),
    Import(Import),
    None,
}

#[derive(Debug)]
pub struct Pipeline {}

impl Pipeline {
    pub fn new() -> Self {
        Self {}
    }
}

impl ParserWorkingSet {
    fn parse_statement(
        &mut self,
        block: &mut Block,
        lite_pipeline: &LiteStatement,
    ) -> Option<ParseError> {
        match lite_pipeline.commands.len() {
            0 => None,
            1 => {
                let command_name = self.get_span_contents(lite_pipeline.commands[0].parts[0]);
                println!("{:?}", command_name);
                if command_name == b"let" {
                    println!("found let")
                }
                None
            }
            _ => {
                // pipeline
                None
            }
        }
    }

    pub fn parse_block(&mut self, lite_block: &LiteBlock) -> (Block, Option<ParseError>) {
        let mut error = None;
        self.enter_scope();

        let mut block = Block::new();

        for pipeline in &lite_block.block {
            let err = self.parse_statement(&mut block, pipeline);
            error = error.or(err);
        }

        self.exit_scope();

        (block, error)
    }

    pub fn parse_file(&mut self, fname: &str, contents: &[u8]) -> (Block, Option<ParseError>) {
        let mut error = None;

        let file_id = self.add_file(fname.into(), contents.into());

        let (output, err) = lex(contents, file_id, 0, crate::LexMode::Normal);
        error = error.or(err);

        let (output, err) = lite_parse(&output);
        error = error.or(err);

        println!("{:?}", output);

        let (output, err) = self.parse_block(&output);
        error = error.or(err);

        (output, error)
    }
}
