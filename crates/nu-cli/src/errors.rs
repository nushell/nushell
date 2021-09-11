use core::ops::Range;

use codespan_reporting::diagnostic::{Diagnostic, Label};
use codespan_reporting::term::termcolor::{ColorChoice, StandardStream};
use nu_parser::ParseError;
use nu_protocol::{engine::StateWorkingSet, ShellError, Span};

fn convert_span_to_diag(
    working_set: &StateWorkingSet,
    span: &Span,
) -> Result<(usize, Range<usize>), Box<dyn std::error::Error>> {
    for (file_id, (_, start, end)) in working_set.files().enumerate() {
        if span.start >= *start && span.end <= *end {
            let new_start = span.start - start;
            let new_end = span.end - start;

            return Ok((file_id, new_start..new_end));
        }
    }

    if span.start == working_set.next_span_start() {
        // We're trying to highlight the space after the end
        if let Some((file_id, (_, _, end))) = working_set.files().enumerate().last() {
            return Ok((file_id, *end..(*end + 1)));
        }
    }

    panic!(
        "internal error: can't find span in parser state: {:?}",
        span
    )
}

pub fn report_parsing_error(
    working_set: &StateWorkingSet,
    error: &ParseError,
) -> Result<(), Box<dyn std::error::Error>> {
    let writer = StandardStream::stderr(ColorChoice::Always);
    let config = codespan_reporting::term::Config::default();

    let diagnostic =
        match error {
            ParseError::Mismatch(expected, found, span) => {
                let (diag_file_id, diag_range) = convert_span_to_diag(working_set, span)?;
                Diagnostic::error()
                    .with_message("Type mismatch during operation")
                    .with_labels(vec![Label::primary(diag_file_id, diag_range)
                        .with_message(format!("expected {}, found {}", expected, found))])
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
            ParseError::KeywordMissingArgument(name, span) => {
                let (diag_file_id, diag_range) = convert_span_to_diag(working_set, span)?;
                Diagnostic::error()
                    .with_message(format!("Missing argument to {}", name))
                    .with_labels(vec![Label::primary(diag_file_id, diag_range)
                        .with_message(format!("missing value that follows {}", name))])
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
            ParseError::MissingColumns(count, span) => {
                let (diag_file_id, diag_range) = convert_span_to_diag(working_set, span)?;
                Diagnostic::error()
                    .with_message("Missing columns")
                    .with_labels(vec![Label::primary(diag_file_id, diag_range).with_message(
                        format!(
                            "expected {} column{}",
                            count,
                            if *count == 1 { "" } else { "s" }
                        ),
                    )])
            }
            ParseError::ExtraColumns(count, span) => {
                let (diag_file_id, diag_range) = convert_span_to_diag(working_set, span)?;
                Diagnostic::error()
                    .with_message("Extra columns")
                    .with_labels(vec![Label::primary(diag_file_id, diag_range).with_message(
                        format!(
                            "expected {} column{}",
                            count,
                            if *count == 1 { "" } else { "s" }
                        ),
                    )])
            }
            ParseError::TypeMismatch(expected, found, span) => {
                let (diag_file_id, diag_range) = convert_span_to_diag(working_set, span)?;
                Diagnostic::error()
                    .with_message("Type mismatch")
                    .with_labels(vec![Label::primary(diag_file_id, diag_range)
                        .with_message(format!("expected {:?}, found {:?}", expected, found))])
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
                    .with_labels(vec![
                        Label::primary(diag_file_id, diag_range).with_message(name.to_string())
                    ])
            }
            ParseError::NonUtf8(span) => {
                let (diag_file_id, diag_range) = convert_span_to_diag(working_set, span)?;
                Diagnostic::error()
                    .with_message("Non-UTF8 code")
                    .with_labels(vec![
                        Label::primary(diag_file_id, diag_range).with_message("non-UTF8 code")
                    ])
            }
            ParseError::Expected(expected, span) => {
                let (diag_file_id, diag_range) = convert_span_to_diag(working_set, span)?;
                Diagnostic::error()
                    .with_message("Parse mismatch during operation")
                    .with_labels(vec![Label::primary(diag_file_id, diag_range)
                        .with_message(format!("expected {}", expected))])
            }
            ParseError::UnsupportedOperation(op_span, lhs_span, lhs_ty, rhs_span, rhs_ty) => {
                let (lhs_file_id, lhs_range) = convert_span_to_diag(working_set, lhs_span)?;
                let (rhs_file_id, rhs_range) = convert_span_to_diag(working_set, rhs_span)?;
                let (op_file_id, op_range) = convert_span_to_diag(working_set, op_span)?;
                Diagnostic::error()
                    .with_message("Unsupported operation")
                    .with_labels(vec![
                        Label::primary(op_file_id, op_range)
                            .with_message("doesn't support these values"),
                        Label::secondary(lhs_file_id, lhs_range).with_message(lhs_ty.to_string()),
                        Label::secondary(rhs_file_id, rhs_range).with_message(rhs_ty.to_string()),
                    ])
            }
            ParseError::ExpectedKeyword(expected, span) => {
                let (diag_file_id, diag_range) = convert_span_to_diag(working_set, span)?;
                Diagnostic::error()
                    .with_message("Expected keyword")
                    .with_labels(vec![Label::primary(diag_file_id, diag_range)
                        .with_message(format!("expected {}", expected))])
            }
            ParseError::IncompleteParser(span) => {
                let (diag_file_id, diag_range) = convert_span_to_diag(working_set, span)?;
                Diagnostic::error()
                    .with_message("Parser incomplete")
                    .with_labels(vec![Label::primary(diag_file_id, diag_range)
                        .with_message("parser support missing for this expression")])
            }
            ParseError::RestNeedsName(span) => {
                let (diag_file_id, diag_range) = convert_span_to_diag(working_set, span)?;
                Diagnostic::error()
                    .with_message("Rest parameter needs a name")
                    .with_labels(vec![Label::primary(diag_file_id, diag_range)
                        .with_message("needs a parameter name")])
            }
        };

    // println!("DIAG");
    // println!("{:?}", diagnostic);
    codespan_reporting::term::emit(&mut writer.lock(), &config, working_set, &diagnostic)?;

    Ok(())
}

pub fn report_shell_error(
    working_set: &StateWorkingSet,
    error: &ShellError,
) -> Result<(), Box<dyn std::error::Error>> {
    let writer = StandardStream::stderr(ColorChoice::Always);
    let config = codespan_reporting::term::Config::default();

    let diagnostic =
        match error {
            ShellError::OperatorMismatch {
                op_span,
                lhs_ty,
                lhs_span,
                rhs_ty,
                rhs_span,
            } => {
                let (lhs_file_id, lhs_range) = convert_span_to_diag(working_set, lhs_span)?;
                let (rhs_file_id, rhs_range) = convert_span_to_diag(working_set, rhs_span)?;
                let (op_file_id, op_range) = convert_span_to_diag(working_set, op_span)?;
                Diagnostic::error()
                    .with_message("Type mismatch during operation")
                    .with_labels(vec![
                        Label::primary(op_file_id, op_range)
                            .with_message("type mismatch for operator"),
                        Label::secondary(lhs_file_id, lhs_range).with_message(lhs_ty.to_string()),
                        Label::secondary(rhs_file_id, rhs_range).with_message(rhs_ty.to_string()),
                    ])
            }
            ShellError::UnsupportedOperator(op, span) => {
                let (diag_file_id, diag_range) = convert_span_to_diag(working_set, span)?;
                Diagnostic::error()
                    .with_message(format!("Unsupported operator: {}", op))
                    .with_labels(vec![Label::primary(diag_file_id, diag_range)
                        .with_message("unsupported operator")])
            }
            ShellError::UnknownOperator(op, span) => {
                let (diag_file_id, diag_range) = convert_span_to_diag(working_set, span)?;
                Diagnostic::error()
                    .with_message(format!("Unsupported operator: {}", op))
                    .with_labels(vec![Label::primary(diag_file_id, diag_range)
                        .with_message("unsupported operator")])
            }
            ShellError::ExternalNotSupported(span) => {
                let (diag_file_id, diag_range) = convert_span_to_diag(working_set, span)?;
                Diagnostic::error()
                    .with_message("External commands not yet supported")
                    .with_labels(vec![Label::primary(diag_file_id, diag_range)
                        .with_message("external not supported")])
            }
            ShellError::InternalError(s) => {
                Diagnostic::error().with_message(format!("Internal error: {}", s))
            }
            ShellError::VariableNotFoundAtRuntime(span) => {
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
            ShellError::CannotCreateRange(span) => {
                let (diag_file_id, diag_range) = convert_span_to_diag(working_set, span)?;
                Diagnostic::error()
                    .with_message("Can't convert range to countable values")
                    .with_labels(vec![Label::primary(diag_file_id, diag_range)
                        .with_message("can't convert to countable values")])
            }
            ShellError::DivisionByZero(span) => {
                let (diag_file_id, diag_range) = convert_span_to_diag(working_set, span)?;

                Diagnostic::error()
                    .with_message("Division by zero")
                    .with_labels(vec![
                        Label::primary(diag_file_id, diag_range).with_message("division by zero")
                    ])
            }
            ShellError::AccessBeyondEnd(len, span) => {
                let (diag_file_id, diag_range) = convert_span_to_diag(working_set, span)?;

                Diagnostic::error()
                    .with_message("Row number too large")
                    .with_labels(vec![Label::primary(diag_file_id, diag_range)
                        .with_message(format!("row number too large (max: {})", *len))])
            }
            ShellError::AccessBeyondEndOfStream(span) => {
                let (diag_file_id, diag_range) = convert_span_to_diag(working_set, span)?;

                Diagnostic::error()
                    .with_message("Row number too large")
                    .with_labels(vec![Label::primary(diag_file_id, diag_range)
                        .with_message("row number too large")])
            }
            ShellError::IncompatiblePathAccess(name, span) => {
                let (diag_file_id, diag_range) = convert_span_to_diag(working_set, span)?;

                Diagnostic::error()
                    .with_message("Data cannot be accessed with a cell path")
                    .with_labels(vec![Label::primary(diag_file_id, diag_range)
                        .with_message(format!("{} doesn't support cell paths", name))])
            }
            ShellError::CantFindColumn(span) => {
                let (diag_file_id, diag_range) = convert_span_to_diag(working_set, span)?;

                //FIXME: add "did you mean"
                Diagnostic::error()
                    .with_message("Cannot find column")
                    .with_labels(vec![
                        Label::primary(diag_file_id, diag_range).with_message("cannot find column")
                    ])
            }
        };

    // println!("DIAG");
    // println!("{:?}", diagnostic);
    codespan_reporting::term::emit(&mut writer.lock(), &config, working_set, &diagnostic)?;

    Ok(())
}
