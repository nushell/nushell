crate mod ast;
crate mod completer;
crate mod lexer;
crate mod parser;
crate mod registry;
crate mod span;

crate use ast::{ParsedCommand, Pipeline};
crate use registry::{Args, CommandConfig};

use crate::errors::ShellError;
use lexer::Lexer;
use parser::PipelineParser;

pub fn parse(input: &str) -> Result<Pipeline, ShellError> {
    let parser = PipelineParser::new();
    let tokens = Lexer::new(input, false);

    match parser.parse(tokens) {
        Ok(val) => Ok(val),
        Err(err) => Err(ShellError::parse_error(err, input.to_string())),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parser::ast::{bare, binary, flag, short, unit, var, Pipeline};
    use pretty_assertions::assert_eq;

    fn assert_parse(source: &str, expected: Pipeline) {
        let parsed = parse(source).unwrap();
        let printed = parsed.print();
        assert_eq!(parsed, expected);
        assert_eq!(source, printed);
    }

    macro_rules! commands {
        ( $( ( $name:tt $( $command:expr)* ) )|* ) => {
            Pipeline::new(vec![
                $(
                    command!($name $($command)*)
                ),*
            ])
        }
    }

    macro_rules! command {
        ($name:ident $( $command:expr )*) => {
            ParsedCommand::new(stringify!($name).into(), vec![ $($command.into()),* ])
        };

        ($name:ident $( $command:expr )*) => {
            ParsedCommand::new(stringify!($name).into(), vec![ $($command.into()),* ])
        };

        ($name:tt $( $command:expr )*) => {
            ParsedCommand::new($name.into(), vec![ $($command.into()),* ])
        };
    }

    #[test]
    fn parse_simple_command() {
        assert_parse("ls", commands![(ls)]);
    }

    #[test]
    fn parse_command_with_args() {
        assert_parse(
            r#"open Cargo.toml | select package.authors | split-row " ""#,
            commands![
                (open bare("Cargo.toml"))
                    | (select bare("package.authors"))
                    | ("split-row" " ")
            ],
        );

        assert_parse(r#"git add ."#, commands![("git" bare("add") bare("."))]);

        assert_parse(
            "open Cargo.toml | select package.version | echo $it",
            commands![
                (open bare("Cargo.toml"))
                    | (select bare("package.version"))
                    | (echo var("it"))
            ],
        );

        assert_parse(
            "open Cargo.toml --raw",
            commands![(open bare("Cargo.toml") flag("raw"))],
        );

        assert_parse(
            "open Cargo.toml -r",
            commands![(open bare("Cargo.toml") short("r"))],
        );

        assert_parse(
            "open Cargo.toml | from-toml | to-toml",
            commands![(open bare("Cargo.toml")) | ("from-toml") | ("to-toml")],
        );

        assert_parse(
            r#"config --get "ignore dups" | format-list"#,
            commands![(config flag("get") "ignore dups") | ("format-list")],
        );

        assert_parse(
            "open Cargo.toml | from-toml | select dependencies | column serde",
            commands![
                (open bare("Cargo.toml"))
                    | ("from-toml")
                    | (select bare("dependencies"))
                    | (column bare("serde"))
            ],
        );

        assert_parse(
            "config --set tabs 2",
            commands![(config flag("set") bare("tabs") 2)],
        );

        assert_parse(
            r#"ls | skip 1 | first 2 | select "file name" | rm $it"#,
            commands![
                (ls)
                    | (skip 1)
                    | (first 2)
                    | (select "file name")
                    | (rm var("it"))
            ],
        );

        assert_parse(
            r#"git branch --merged | split-row "`n" | where $it != "* master""#,
            commands![
                // TODO: Handle escapes correctly. Should we do ` escape because of paths?
                (git bare("branch") flag("merged")) | ("split-row" "`n") | (where binary(var("it"), "!=", "* master"))
            ],
        );

        assert_parse(
            r#"open input2.json | from-json | select glossary.GlossDiv.GlossList.GlossEntry.GlossDef.GlossSeeAlso | where $it > "GML""#,
            commands![
                (open bare("input2.json"))
                    | ("from-json")
                    | (select bare("glossary.GlossDiv.GlossList.GlossEntry.GlossDef.GlossSeeAlso"))
                    | (where binary(var("it"), ">", "GML"))
            ]
        );

        assert_parse(
            r"cd ..\.cargo\",
            commands![
                (cd bare(r"..\.cargo\"))
            ],
        );

        assert_parse(
            "ls | where size < 1KB",
            commands![
                (ls) | (where binary(bare("size"), "<", unit(1, "KB")))
            ],
        );
    }
}
