use core::ops::Range;

use crate::{ParseError, ParserWorkingSet, ShellError, Span};
use codespan_reporting::diagnostic::{Diagnostic, Label};
use codespan_reporting::term::termcolor::{ColorChoice, StandardStream};

impl<'a> codespan_reporting::files::Files<'a> for ParserWorkingSet<'a> {
    type FileId = usize;

    type Name = String;

    type Source = String;

    fn name(&'a self, id: Self::FileId) -> Result<Self::Name, codespan_reporting::files::Error> {
        Ok(self.get_filename(id))
    }

    fn source(
        &'a self,
        id: Self::FileId,
    ) -> Result<Self::Source, codespan_reporting::files::Error> {
        Ok(self.get_file_source(id))
    }

    fn line_index(
        &'a self,
        id: Self::FileId,
        byte_index: usize,
    ) -> Result<usize, codespan_reporting::files::Error> {
        let source = self.get_file_source(id);

        let mut count = 0;

        for byte in source.bytes().enumerate() {
            if byte.0 == byte_index {
                // println!("count: {} for file: {} index: {}", count, id, byte_index);
                return Ok(count);
            }
            if byte.1 == b'\n' {
                count += 1;
            }
        }

        // println!("count: {} for file: {} index: {}", count, id, byte_index);
        Ok(count)
    }

    fn line_range(
        &'a self,
        id: Self::FileId,
        line_index: usize,
    ) -> Result<Range<usize>, codespan_reporting::files::Error> {
        let source = self.get_file_source(id);

        let mut count = 0;

        let mut start = Some(0);
        let mut end = None;

        for byte in source.bytes().enumerate() {
            #[allow(clippy::comparison_chain)]
            if count > line_index {
                let start = start.expect("internal error: couldn't find line");
                let end = end.expect("internal error: couldn't find line");

                // println!(
                //     "Span: {}..{} for fileid: {} index: {}",
                //     start, end, id, line_index
                // );
                return Ok(start..end);
            } else if count == line_index {
                end = Some(byte.0 + 1);
            }

            #[allow(clippy::comparison_chain)]
            if byte.1 == b'\n' {
                count += 1;
                if count > line_index {
                    break;
                } else if count == line_index {
                    start = Some(byte.0 + 1);
                }
            }
        }

        match (start, end) {
            (Some(start), Some(end)) => {
                // println!(
                //     "Span: {}..{} for fileid: {} index: {}",
                //     start, end, id, line_index
                // );
                Ok(start..end)
            }
            _ => Err(codespan_reporting::files::Error::FileMissing),
        }
    }
}

fn convert_span_to_diag(
    working_set: &ParserWorkingSet,
    span: &Span,
) -> Result<(usize, Range<usize>), Box<dyn std::error::Error>> {
    for (file_id, (_, start, end)) in working_set.files().enumerate() {
        if span.start >= *start && span.end <= *end {
            let new_start = span.start - start;
            let new_end = span.end - start;

            return Ok((file_id, new_start..new_end));
        }
    }

    panic!("internal error: can't find span in parser state")
}

pub fn report_parsing_error(
    working_set: &ParserWorkingSet,
    error: &ParseError,
) -> Result<(), Box<dyn std::error::Error>> {
    let writer = StandardStream::stderr(ColorChoice::Always);
    let config = codespan_reporting::term::Config::default();

    let diagnostic =
        match error {
            ParseError::Mismatch(missing, span) => {
                let (diag_file_id, diag_range) = convert_span_to_diag(working_set, span)?;
                Diagnostic::error()
                    .with_message("Type mismatch during operation")
                    .with_labels(vec![Label::primary(diag_file_id, diag_range)
                        .with_message(format!("expected {}", missing))])
            }
            ParseError::ExtraTokens(span) => {
                let (diag_file_id, diag_range) = convert_span_to_diag(working_set, span)?;
                Diagnostic::error()
                    .with_message("Extra tokens in code")
                    .with_labels(vec![
                        Label::primary(diag_file_id, diag_range).with_message("extra tokens")
                    ])
            }
            ParseError::ExtraPositional(span) => {
                let (diag_file_id, diag_range) = convert_span_to_diag(working_set, span)?;
                Diagnostic::error()
                    .with_message("Extra positional argument")
                    .with_labels(vec![Label::primary(diag_file_id, diag_range)
                        .with_message("extra positional argument")])
            }
            ParseError::UnexpectedEof(s, span) => {
                let (diag_file_id, diag_range) = convert_span_to_diag(working_set, span)?;
                Diagnostic::error()
                    .with_message("Unexpected end of code")
                    .with_labels(vec![Label::primary(diag_file_id, diag_range)
                        .with_message(format!("expected {}", s))])
            }
            ParseError::Unclosed(delim, span) => {
                let (diag_file_id, diag_range) = convert_span_to_diag(working_set, span)?;
                Diagnostic::error()
                    .with_message("Unclosed delimiter")
                    .with_labels(vec![Label::primary(diag_file_id, diag_range)
                        .with_message(format!("unclosed {}", delim))])
            }
            ParseError::UnknownStatement(span) => {
                let (diag_file_id, diag_range) = convert_span_to_diag(working_set, span)?;
                Diagnostic::error()
                    .with_message("Unknown statement")
                    .with_labels(vec![
                        Label::primary(diag_file_id, diag_range).with_message("unknown statement")
                    ])
            }
            ParseError::MultipleRestParams(span) => {
                let (diag_file_id, diag_range) = convert_span_to_diag(working_set, span)?;
                Diagnostic::error()
                    .with_message("Multiple rest params")
                    .with_labels(vec![Label::primary(diag_file_id, diag_range)
                        .with_message("multiple rest params")])
            }
            ParseError::VariableNotFound(span) => {
                let (diag_file_id, diag_range) = convert_span_to_diag(working_set, span)?;
                Diagnostic::error()
                    .with_message("Variable not found")
                    .with_labels(vec![
                        Label::primary(diag_file_id, diag_range).with_message("variable not found")
                    ])
            }
            ParseError::UnknownCommand(span) => {
                let (diag_file_id, diag_range) = convert_span_to_diag(working_set, span)?;
                Diagnostic::error()
                    .with_message("Unknown command")
                    .with_labels(vec![
                        Label::primary(diag_file_id, diag_range).with_message("unknown command")
                    ])
            }
            ParseError::UnknownFlag(span) => {
                let (diag_file_id, diag_range) = convert_span_to_diag(working_set, span)?;
                Diagnostic::error()
                    .with_message("Unknown flag")
                    .with_labels(vec![
                        Label::primary(diag_file_id, diag_range).with_message("unknown flag")
                    ])
            }
            ParseError::UnknownType(span) => {
                let (diag_file_id, diag_range) = convert_span_to_diag(working_set, span)?;
                Diagnostic::error()
                    .with_message("Unknown type")
                    .with_labels(vec![
                        Label::primary(diag_file_id, diag_range).with_message("unknown type")
                    ])
            }
            ParseError::MissingFlagParam(span) => {
                let (diag_file_id, diag_range) = convert_span_to_diag(working_set, span)?;
                Diagnostic::error()
                    .with_message("Missing flag param")
                    .with_labels(vec![Label::primary(diag_file_id, diag_range)
                        .with_message("flag missing parameter")])
            }
            ParseError::ShortFlagBatchCantTakeArg(span) => {
                let (diag_file_id, diag_range) = convert_span_to_diag(working_set, span)?;
                Diagnostic::error()
                    .with_message("Batches of short flags can't take arguments")
                    .with_labels(vec![Label::primary(diag_file_id, diag_range)
                        .with_message("short flag batches can't take args")])
            }
            ParseError::MissingPositional(name, span) => {
                let (diag_file_id, diag_range) = convert_span_to_diag(working_set, span)?;
                Diagnostic::error()
                    .with_message("Missing required positional arg")
                    .with_labels(vec![Label::primary(diag_file_id, diag_range)
                        .with_message(format!("missing {}", name))])
            }
            ParseError::MissingType(span) => {
                let (diag_file_id, diag_range) = convert_span_to_diag(working_set, span)?;
                Diagnostic::error()
                    .with_message("Missing type")
                    .with_labels(vec![
                        Label::primary(diag_file_id, diag_range).with_message("expected type")
                    ])
            }
            ParseError::TypeMismatch(ty, span) => {
                let (diag_file_id, diag_range) = convert_span_to_diag(working_set, span)?;
                Diagnostic::error()
                    .with_message("Type mismatch")
                    .with_labels(vec![Label::primary(diag_file_id, diag_range)
                        .with_message(format!("expected {:?}", ty))])
            }
            ParseError::MissingRequiredFlag(name, span) => {
                let (diag_file_id, diag_range) = convert_span_to_diag(working_set, span)?;
                Diagnostic::error()
                    .with_message("Missing required flag")
                    .with_labels(vec![Label::primary(diag_file_id, diag_range)
                        .with_message(format!("missing required flag {}", name))])
            }
            ParseError::IncompleteMathExpression(span) => {
                let (diag_file_id, diag_range) = convert_span_to_diag(working_set, span)?;
                Diagnostic::error()
                    .with_message("Incomplete math expresssion")
                    .with_labels(vec![Label::primary(diag_file_id, diag_range)
                        .with_message("incomplete math expression")])
            }
            ParseError::UnknownState(name, span) => {
                let (diag_file_id, diag_range) = convert_span_to_diag(working_set, span)?;
                Diagnostic::error()
                    .with_message("Unknown state")
                    .with_labels(vec![Label::primary(diag_file_id, diag_range)
                        .with_message(format!("unknown state {}", name))])
            }
            ParseError::NonUtf8(span) => {
                let (diag_file_id, diag_range) = convert_span_to_diag(working_set, span)?;
                Diagnostic::error()
                    .with_message("Non-UTF8 code")
                    .with_labels(vec![
                        Label::primary(diag_file_id, diag_range).with_message("non-UTF8 code")
                    ])
            }
        };

    // println!("DIAG");
    // println!("{:?}", diagnostic);
    codespan_reporting::term::emit(&mut writer.lock(), &config, working_set, &diagnostic)?;

    Ok(())
}

pub fn report_shell_error(
    working_set: &ParserWorkingSet,
    error: &ShellError,
) -> Result<(), Box<dyn std::error::Error>> {
    let writer = StandardStream::stderr(ColorChoice::Always);
    let config = codespan_reporting::term::Config::default();

    let diagnostic = match error {
        ShellError::OperatorMismatch(operator, ty1, span1, ty2, span2) => {
            let (diag_file_id1, diag_range1) = convert_span_to_diag(working_set, span1)?;
            let (diag_file_id2, diag_range2) = convert_span_to_diag(working_set, span2)?;
            Diagnostic::error()
                .with_message(format!("Type mismatch during operation '{}'", operator))
                .with_labels(vec![
                    Label::primary(diag_file_id1, diag_range1).with_message(ty1.to_string()),
                    Label::secondary(diag_file_id2, diag_range2).with_message(ty2.to_string()),
                ])
        }
        ShellError::Unsupported(span) => {
            let (diag_file_id, diag_range) = convert_span_to_diag(working_set, span)?;
            Diagnostic::error()
                .with_message("Unsupported operation")
                .with_labels(vec![
                    Label::primary(diag_file_id, diag_range).with_message("unsupported operation")
                ])
        }
        ShellError::InternalError(s) => {
            Diagnostic::error().with_message(format!("Internal error: {}", s))
        }
        ShellError::VariableNotFound(span) => {
            let (diag_file_id, diag_range) = convert_span_to_diag(working_set, span)?;
            Diagnostic::error()
                .with_message("Variable not found")
                .with_labels(vec![
                    Label::primary(diag_file_id, diag_range).with_message("variable not found")
                ])
        }
        ShellError::CantConvert(s, span) => {
            let (diag_file_id, diag_range) = convert_span_to_diag(working_set, span)?;
            Diagnostic::error()
                .with_message(format!("Can't convert to {}", s))
                .with_labels(vec![Label::primary(diag_file_id, diag_range)
                    .with_message(format!("can't convert to {}", s))])
        }
    };

    // println!("DIAG");
    // println!("{:?}", diagnostic);
    codespan_reporting::term::emit(&mut writer.lock(), &config, working_set, &diagnostic)?;

    Ok(())
}
