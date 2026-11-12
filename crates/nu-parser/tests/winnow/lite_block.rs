//! [`LiteBlock`] against nu-parser's own `lex` and `lite_parse`, which decide what is right: the
//! winnow lexer keeps the ends of line nu's folds into a `|`, so this checks that rule too.

use nu_parser::winnow::syntax::LiteBlock;
use nu_parser::winnow::syntax::Span;
use nu_parser::winnow::syntax::ast::LiteError;
use nu_parser::winnow::syntax::lex::{
    GroupEnds, LexOptions, RedirectionSource, TokenContents, lex_recovering,
};
use nu_protocol::ParseError;
use nu_protocol::ast::RedirectionSource as NuSource;
use nu_protocol::engine::{EngineState, StateWorkingSet};
use std::path::Path;

type At = (usize, usize);

/// A command as either lite parser read it, spans counted from the start of the text.
#[derive(Debug, PartialEq, Eq)]
struct ReadCommand {
    parts: Vec<At>,
    comments: Vec<At>,
    pipe: Option<At>,
    attribute_ends: Vec<usize>,
    /// Each redirection's operator and file.
    redirections: Vec<At>,
}

/// A block as either lite parser read it.
#[derive(Debug, PartialEq, Eq)]
struct ReadBlock {
    pipelines: Vec<Vec<ReadCommand>>,
    error: Option<String>,
}

/// The text's block as nu's own `lex` and `lite_parse` read it.
fn read_by_nu(engine_state: &EngineState, text: &str) -> ReadBlock {
    let mut working_set = StateWorkingSet::new(engine_state);
    let offset = working_set.next_span_start();
    let _file = working_set.add_file("lite-block-test", text.as_bytes());
    let at = |span: nu_protocol::Span| (span.start - offset, span.end - offset);
    let (tokens, _) = nu_parser::lex(text.as_bytes(), offset, &[], &[], false);
    let (block, error) = nu_parser::lite_parse(&tokens, &working_set);
    let pipelines = block
        .block
        .iter()
        .map(|pipeline| {
            pipeline
                .commands
                .iter()
                .map(|command| {
                    let mut redirections: Vec<At> = command
                        .redirection
                        .iter()
                        .flat_map(|redirection| redirection.spans())
                        .map(at)
                        .collect();
                    redirections.sort_unstable();
                    ReadCommand {
                        parts: command.parts.iter().map(|span| at(*span)).collect(),
                        comments: command.comments.iter().map(|span| at(*span)).collect(),
                        pipe: command.pipe.map(at),
                        attribute_ends: command.attribute_idx.clone(),
                        redirections,
                    }
                })
                .collect()
        })
        .collect();
    let error = error.map(|error| match error {
        ParseError::ShellOrOr(span) => format!("or {:?}", at(span)),
        ParseError::UnexpectedEof(_, span) => format!("end {:?}", at(span)),
        ParseError::Expected(_, span) => format!("target {:?}", at(span)),
        ParseError::UnexpectedRedirection { span } => format!("nothing {:?}", at(span)),
        ParseError::MultipleRedirections(source, first, second) => {
            let source = match source {
                NuSource::Stdout => "out",
                NuSource::Stderr => "err",
                NuSource::StdoutAndStderr => "both",
            };
            format!("twice {source} {:?} {:?}", at(first), at(second))
        }
        other => format!("{other:?}"),
    });
    ReadBlock { pipelines, error }
}

/// The text's block as the winnow lexer and [`LiteBlock`] read it, `None` when the lexer gives
/// up on the text.
fn read_by_winnow(text: &str) -> Option<ReadBlock> {
    let span = Span::new(0, text.len());
    let lexed = lex_recovering(
        text,
        0,
        LexOptions::BLOCK,
        usize::MAX,
        &mut GroupEnds::new(span),
    )
    .ok()?;
    let tokens = match lexed.tokens.split_last() {
        Some((eof, rest)) if eof.contents == TokenContents::Eof => rest,
        _ => lexed.tokens.as_slice(),
    };
    let block = LiteBlock::parse(text, tokens);
    let at = |span: Span| (span.start, span.end);
    let pipelines = block
        .pipelines()
        .map(|pipeline| {
            pipeline
                .iter()
                .map(|command| {
                    let mut redirections: Vec<At> = block
                        .redirections(command)
                        .iter()
                        .flat_map(|(operator, file)| {
                            std::iter::once(operator.span).chain(file.map(|file| file.span))
                        })
                        .map(at)
                        .collect();
                    redirections.sort_unstable();
                    ReadCommand {
                        parts: block
                            .parts(command)
                            .iter()
                            .map(|token| at(token.span))
                            .collect(),
                        comments: block.comments(command).iter().map(|s| at(*s)).collect(),
                        pipe: command.pipe.map(at),
                        attribute_ends: block.attribute_ends(command).collect(),
                        redirections,
                    }
                })
                .collect()
        })
        .collect();
    let error = block.error().map(|error| match error {
        LiteError::ShellOrOr(span) => format!("or {:?}", at(span)),
        LiteError::PipelineMissingEnd(span) => format!("end {:?}", at(span)),
        LiteError::MissingRedirectionTarget(span) => format!("target {:?}", at(span)),
        LiteError::NothingToRedirect(span) => format!("nothing {:?}", at(span)),
        LiteError::MultipleRedirections {
            source,
            first,
            second,
        } => {
            let source = match source {
                RedirectionSource::Stdout => "out",
                RedirectionSource::Stderr => "err",
                RedirectionSource::StdoutAndStderr => "both",
            };
            format!("twice {source} {:?} {:?}", at(first), at(second))
        }
    });
    Some(ReadBlock { pipelines, error })
}

/// Compares each text's reading by the two, and panics with the first few that differ.
#[derive(Default)]
struct Comparison {
    compared: usize,
    lexer_gave_up: usize,
    differ: usize,
    first: Vec<String>,
}

impl Comparison {
    fn check(&mut self, engine_state: &EngineState, text: &str) {
        let Some(winnow) = read_by_winnow(text) else {
            self.lexer_gave_up += 1;
            return;
        };
        self.compared += 1;
        let nu = read_by_nu(engine_state, text);
        if nu != winnow {
            self.differ += 1;
            if self.first.len() < 25 {
                self.first
                    .push(format!("{text:?}\n  nu     {nu:?}\n  winnow {winnow:?}"));
            }
        }
    }

    fn assert_none_differ(self) {
        println!(
            "{} compared, {} the lexer gave up on",
            self.compared, self.lexer_gave_up
        );
        assert!(
            self.differ == 0,
            "{} of {} differ ({} the lexer gave up on):\n{}",
            self.differ,
            self.compared,
            self.lexer_gave_up,
            self.first.join("\n")
        );
        assert!(self.compared > 0);
    }
}

/// Calls `each` with every run of `alphabet`'s words up to `longest` long, as they were
/// joined by `join`.
fn every_run(alphabet: &[&str], longest: usize, each: &mut dyn FnMut(&[&str])) {
    fn extend<'w>(
        alphabet: &[&'w str],
        left: usize,
        run: &mut Vec<&'w str>,
        each: &mut dyn FnMut(&[&str]),
    ) {
        each(run);
        if left == 0 {
            return;
        }
        for word in alphabet {
            run.push(word);
            extend(alphabet, left - 1, run, each);
            run.pop();
        }
    }
    extend(alphabet, longest, &mut Vec::new(), each);
}

/// Short texts made to reach every path of the lite parser, with runs up to these lengths:
/// the delimiters and a word, then pipes, redirections and comments between commands, then
/// comments, pipes and line ends before a definition.
fn check_short_texts(chars: usize, pipes: usize, comments: usize) -> Comparison {
    let engine_state = EngineState::new();
    let mut comparison = Comparison::default();
    let delimiters = [
        "(", ")", "[", "]", "{", "}", "\"", "'", "|", ";", "$", ".", ":", "=", "#", "\n", " ", "a",
        "1", "-", "@",
    ];
    every_run(&delimiters, chars, &mut |run| {
        comparison.check(&engine_state, &run.concat());
    });
    let between = [
        "|", "e>|", "o+e>|", "# c", "\n", ";", "o> f", "e> f", "||", "=", "@a", "o>", "o+e>",
    ];
    for head in ["^ls", "ls"] {
        for tail in ["lines", ""] {
            every_run(&between, pipes, &mut |run| {
                let mut text = head.to_string();
                for word in run {
                    text.push(' ');
                    text.push_str(word);
                    // A comment runs to the end of its line.
                    if word.starts_with('#') {
                        text.push('\n');
                    }
                }
                text.push(' ');
                text.push_str(tail);
                comparison.check(&engine_state, text.trim_end_matches(' '));
            });
        }
    }
    let before = ["# a", "#", "|", ";", "1", "\n", "@a", "o>"];
    every_run(&before, comments, &mut |run| {
        let mut text = String::new();
        for word in run {
            text.push_str(word);
            if word.starts_with('#') {
                text.push('\n');
            } else if *word != "\n" {
                text.push(' ');
            }
        }
        text.push_str("def f [] {}");
        comparison.check(&engine_state, &text);
    });
    comparison
}

fn check_files(dir: &Path, engine_state: &EngineState, comparison: &mut Comparison) {
    let Ok(entries) = std::fs::read_dir(dir) else {
        return;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_dir() {
            check_files(&path, engine_state, comparison);
        } else if path.extension().is_some_and(|extension| extension == "nu")
            && let Ok(text) = std::fs::read_to_string(&path)
        {
            comparison.check(engine_state, &text);
        }
    }
}

#[test]
fn reads_short_texts_as_nu_does() {
    check_short_texts(3, 3, 4).assert_none_differ();
}

/// The same runs, longer: minutes in a debug build.
#[test]
#[ignore = "takes minutes"]
fn reads_longer_short_texts_as_nu_does() {
    check_short_texts(4, 5, 6).assert_none_differ();
}

#[test]
fn reads_the_fixtures_as_nu_does() {
    let manifest = Path::new(env!("CARGO_MANIFEST_DIR"));
    let engine_state = EngineState::new();
    let mut comparison = Comparison::default();
    check_files(
        &manifest.join("tests/winnow"),
        &engine_state,
        &mut comparison,
    );
    check_files(&manifest.join("../nu-std"), &engine_state, &mut comparison);
    comparison.assert_none_differ();
}

#[test]
fn a_long_run_of_comment_lines_folds_into_the_pipe_after_it() {
    let block = |text: &str| read_by_winnow(text).expect("the text lexes");
    let folded = block("ls\n# a\n# b\n| lines");
    assert_eq!(folded.pipelines.len(), 1);
    assert_eq!(folded.pipelines[0].len(), 2);
    let not_folded = block("ls\n# a\n\n# b\n| lines");
    assert_eq!(not_folded.pipelines.len(), 2);
}

#[test]
fn a_double_ampersand_folds_no_line_end() {
    // nu's lexer reads `&&` apart from `|`, though both are pipes to the lite parser.
    let block = read_by_winnow("ls\n&& lines").expect("the text lexes");
    assert_eq!(block.pipelines.len(), 2);
}
