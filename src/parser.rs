crate mod ast;
crate mod completer;
crate mod lexer;
crate mod parser;
crate mod registry;
crate mod span;
crate mod parse2;

crate use ast::Pipeline;
crate use registry::{Args, CommandConfig};

use crate::errors::ShellError;
use ast::Module;
use lexer::Lexer;
use log::trace;
use parser::{ModuleParser, ReplLineParser};

pub fn parse(input: &str) -> Result<Pipeline, ShellError> {
    let _ = pretty_env_logger::try_init();

    let parser = ReplLineParser::new();
    let tokens = Lexer::new(input, false);

    trace!(
        "Tokens: {:?}",
        tokens.clone().collect::<Result<Vec<_>, _>>()
    );

    match parser.parse(tokens) {
        Ok(val) => Ok(val),
        Err(err) => Err(ShellError::parse_error(err)),
    }
}

#[allow(unused)]
pub fn parse_module(input: &str) -> Result<Module, ShellError> {
    let _ = pretty_env_logger::try_init();

    let parser = ModuleParser::new();
    let tokens = Lexer::new(input, false);

    trace!(
        "Tokens: {:?}",
        tokens.clone().collect::<Result<Vec<_>, _>>()
    );

    match parser.parse(tokens) {
        Ok(val) => Ok(val),
        Err(err) => Err(ShellError::parse_error(err)),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parser::ast::Pipeline;
    use pretty_assertions::assert_eq;

    fn assert_parse(source: &str, expected: Pipeline) {
        let parsed = match parse(source) {
            Ok(p) => p,
            Err(ShellError::Diagnostic(diag)) => {
                use language_reporting::termcolor;

                let writer = termcolor::StandardStream::stdout(termcolor::ColorChoice::Auto);
                let files = crate::parser::span::Files::new(source.to_string());

                language_reporting::emit(
                    &mut writer.lock(),
                    &files,
                    &diag.diagnostic,
                    &language_reporting::DefaultConfig,
                )
                .unwrap();

                panic!("Test failed")
            }
            Err(err) => panic!("Something went wrong during parse: {:#?}", err),
        };

        let printed = parsed.print();

        assert_eq!(parsed, expected);
        assert_eq!(printed, source);

        let span = expected.span;

        let expected_module = ModuleBuilder::spanned_items(
            vec![Spanned::from_item(RawItem::Expression(expected), span)],
            span.start,
            span.end,
        );

        assert_parse_module(source, expected_module);
    }

    fn assert_parse_module(source: &str, expected: Module) {
        let parsed = match parse_module(source) {
            Ok(p) => p,
            Err(ShellError::Diagnostic(diag)) => {
                use language_reporting::termcolor;

                let writer = termcolor::StandardStream::stdout(termcolor::ColorChoice::Auto);
                let files = crate::parser::span::Files::new(source.to_string());

                language_reporting::emit(
                    &mut writer.lock(),
                    &files,
                    &diag.diagnostic,
                    &language_reporting::DefaultConfig,
                )
                .unwrap();

                panic!("Test failed")
            }
            Err(err) => panic!("Something went wrong during parse: {:#?}", err),
        };

        let printed = parsed.print();

        assert_eq!(parsed, expected);
        assert_eq!(printed, source);
    }

    macro_rules! commands {
        ( $( ( $name:tt $( $command:ident ( $arg:expr ) )* ) )|* ) => {{
            use $crate::parser::ast::{Expression, ExpressionBuilder};
            let mut builder = crate::parser::ast::ExpressionBuilder::new();

            builder.pipeline(vec![
                $(
                    (command!($name $($command($arg))*) as (&dyn Fn(&mut ExpressionBuilder) -> Expression))
                ),*
            ])
        }}
    }

    macro_rules! command {
        ($name:ident) => {
            &|b: &mut $crate::parser::ast::ExpressionBuilder| b.call((
                &|b: &mut $crate::parser::ast::ExpressionBuilder| b.bare(stringify!($name)),
                vec![]
            ))
        };

        ($name:ident $( $command:ident ( $body:expr ) )*) => {{
            use $crate::parser::ast::{Expression, ExpressionBuilder};
            &|b: &mut ExpressionBuilder| b.call((
                (&|b: &mut ExpressionBuilder| b.bare(stringify!($name))) as (&dyn Fn(&mut ExpressionBuilder) -> Expression),
                vec![$( (&|b: &mut ExpressionBuilder| b.$command($body)) as &dyn Fn(&mut ExpressionBuilder) -> Expression ),* ]))

        }};

        ($name:ident $( $command:ident ( $body:expr ) )*) => {
            &|b: &mut $crate::parser::ast::ExpressionBuilder| b.call(|b| b.bare(stringify!($name)), vec![ $( |b| b.$command($body) ),* ])
        };

        ($name:tt $( $command:ident ( $body:expr ) )*) => {
            &|b: &mut $crate::parser::ast::ExpressionBuilder| b.call((&|b| b.bare($name), vec![ $( &|b| b.$command($body) ),* ]))
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
                    | ("split-row" string(" "))
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
            commands![(open bare("Cargo.toml") shorthand("r"))],
        );

        assert_parse(
            "open Cargo.toml | from-toml | to-toml",
            commands![(open bare("Cargo.toml")) | ("from-toml") | ("to-toml")],
        );

        assert_parse(
            r#"config --get "ignore dups" | format-list"#,
            commands![(config flag("get") string("ignore dups")) | ("format-list")],
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
            commands![(config flag("set") bare("tabs") int(2))],
        );

        assert_parse(
            r#"ls | skip 1 | first 2 | select "file name" | rm $it"#,
            commands![
                (ls)
                    | (skip int(1))
                    | (first int(2))
                    | (select string("file name"))
                    | (rm var("it"))
            ],
        );

        assert_parse(
            r#"git branch --merged | split-row "`n" | where $it != "* master""#,
            commands![
                // TODO: Handle escapes correctly. Should we do ` escape because of paths?
                (git bare("branch") flag("merged")) | ("split-row" string("`n")) | (where binary((&|b| b.var("it"), &|b| b.op("!="), &|b| b.string("* master"))))
            ],
        );

        assert_parse(
            r#"open input2.json | from-json | select glossary.GlossDiv.GlossList.GlossEntry.GlossDef.GlossSeeAlso | where $it > "GML""#,
            commands![
                (open bare("input2.json"))
                    | ("from-json")
                    | (select bare("glossary.GlossDiv.GlossList.GlossEntry.GlossDef.GlossSeeAlso"))
                    | (where binary((&|b| b.var("it"), &|b| b.op(">"), &|b| b.string("GML"))))
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
                (ls) | (where binary((&|b| b.bare("size"), &|b| b.op("<"), &|b| b.unit((1, "KB")))))
            ],
        );

        assert_parse(
            "ls | where { $it.size > 100 }",
            commands![
                (ls) | (where block(&|b| b.binary((&|b| b.path((&|b| b.var("it"), vec!["size"])), &|b| b.op(">"), &|b| b.int(100)))))
            ],
        )
    }

    use crate::parser::ast::{ModuleBuilder, RawItem};
    use crate::parser::lexer::Spanned;

}
