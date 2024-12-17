#![allow(clippy::byte_char_slices)]

use crate::{
    lex, lite_parse,
    parser::{is_variable, parse_value},
};
use nu_protocol::{
    ast::{MatchPattern, Pattern},
    engine::StateWorkingSet,
    ParseError, Span, SyntaxShape, Type, VarId,
};
pub fn garbage(span: Span) -> MatchPattern {
    MatchPattern {
        pattern: Pattern::Garbage,
        guard: None,
        span,
    }
}

pub fn parse_pattern(working_set: &mut StateWorkingSet, span: Span) -> MatchPattern {
    let bytes = working_set.get_span_contents(span);

    if bytes.starts_with(b"$") {
        // Variable pattern
        parse_variable_pattern(working_set, span)
    } else if bytes.starts_with(b"{") {
        // Record pattern
        parse_record_pattern(working_set, span)
    } else if bytes.starts_with(b"[") {
        // List pattern
        parse_list_pattern(working_set, span)
    } else if bytes == b"_" {
        MatchPattern {
            pattern: Pattern::IgnoreValue,
            guard: None,
            span,
        }
    } else {
        // Literal value
        let value = parse_value(working_set, span, &SyntaxShape::Any);

        MatchPattern {
            pattern: Pattern::Value(Box::new(value)),
            guard: None,
            span,
        }
    }
}

fn parse_variable_pattern_helper(working_set: &mut StateWorkingSet, span: Span) -> Option<VarId> {
    let bytes = working_set.get_span_contents(span);

    if is_variable(bytes) {
        if let Some(var_id) = working_set.find_variable_in_current_frame(bytes) {
            Some(var_id)
        } else {
            let var_id = working_set.add_variable(bytes.to_vec(), span, Type::Any, false);

            Some(var_id)
        }
    } else {
        None
    }
}

pub fn parse_variable_pattern(working_set: &mut StateWorkingSet, span: Span) -> MatchPattern {
    if let Some(var_id) = parse_variable_pattern_helper(working_set, span) {
        MatchPattern {
            pattern: Pattern::Variable(var_id),
            guard: None,
            span,
        }
    } else {
        working_set.error(ParseError::Expected("valid variable name", span));
        garbage(span)
    }
}

pub fn parse_list_pattern(working_set: &mut StateWorkingSet, span: Span) -> MatchPattern {
    let bytes = working_set.get_span_contents(span);

    let mut start = span.start;
    let mut end = span.end;

    if bytes.starts_with(b"[") {
        start += 1;
    }
    if bytes.ends_with(b"]") {
        end -= 1;
    } else {
        working_set.error(ParseError::Unclosed("]".into(), Span::new(end, end)));
    }

    let inner_span = Span::new(start, end);
    let source = working_set.get_span_contents(inner_span);

    let (output, err) = lex(source, inner_span.start, &[b'\n', b'\r', b','], &[], true);
    if let Some(err) = err {
        working_set.error(err);
    }

    let (output, err) = lite_parse(&output);
    if let Some(err) = err {
        working_set.error(err);
    }

    let mut args = vec![];

    if !output.block.is_empty() {
        for command in &output.block[0].commands {
            let mut spans_idx = 0;

            while spans_idx < command.parts.len() {
                let contents = working_set.get_span_contents(command.parts[spans_idx]);
                if contents == b".." {
                    args.push(MatchPattern {
                        pattern: Pattern::IgnoreRest,
                        guard: None,
                        span: command.parts[spans_idx],
                    });
                    break;
                } else if contents.starts_with(b"..$") {
                    if let Some(var_id) = parse_variable_pattern_helper(
                        working_set,
                        Span::new(
                            command.parts[spans_idx].start + 2,
                            command.parts[spans_idx].end,
                        ),
                    ) {
                        args.push(MatchPattern {
                            pattern: Pattern::Rest(var_id),
                            guard: None,
                            span: command.parts[spans_idx],
                        });
                        break;
                    } else {
                        args.push(garbage(command.parts[spans_idx]));
                        working_set.error(ParseError::Expected(
                            "valid variable name",
                            command.parts[spans_idx],
                        ));
                    }
                } else {
                    let arg = parse_pattern(working_set, command.parts[spans_idx]);

                    args.push(arg);
                };

                spans_idx += 1;
            }
        }
    }

    MatchPattern {
        pattern: Pattern::List(args),
        guard: None,
        span,
    }
}

pub fn parse_record_pattern(working_set: &mut StateWorkingSet, span: Span) -> MatchPattern {
    let mut bytes = working_set.get_span_contents(span);

    let mut start = span.start;
    let mut end = span.end;

    if bytes.starts_with(b"{") {
        start += 1;
    } else {
        working_set.error(ParseError::Expected("{", Span::new(start, start + 1)));
        bytes = working_set.get_span_contents(span);
    }

    if bytes.ends_with(b"}") {
        end -= 1;
    } else {
        working_set.error(ParseError::Unclosed("}".into(), Span::new(end, end)));
    }

    let inner_span = Span::new(start, end);
    let source = working_set.get_span_contents(inner_span);

    let (tokens, err) = lex(source, start, &[b'\n', b'\r', b','], &[b':'], true);
    if let Some(err) = err {
        working_set.error(err);
    }

    let mut output = vec![];
    let mut idx = 0;

    while idx < tokens.len() {
        let bytes = working_set.get_span_contents(tokens[idx].span);
        let (field, pattern) = if !bytes.is_empty() && bytes[0] == b'$' {
            // If this is a variable, treat it as both the name of the field and the pattern
            let field = String::from_utf8_lossy(&bytes[1..]).to_string();

            let pattern = parse_variable_pattern(working_set, tokens[idx].span);

            (field, pattern)
        } else {
            let field = String::from_utf8_lossy(bytes).to_string();

            idx += 1;
            if idx == tokens.len() {
                working_set.error(ParseError::Expected("record", span));
                return garbage(span);
            }
            let colon = working_set.get_span_contents(tokens[idx].span);
            idx += 1;
            if idx == tokens.len() || colon != b":" {
                //FIXME: need better error
                working_set.error(ParseError::Expected("record", span));
                return garbage(span);
            }
            let pattern = parse_pattern(working_set, tokens[idx].span);

            (field, pattern)
        };
        idx += 1;

        output.push((field, pattern));
    }

    MatchPattern {
        pattern: Pattern::Record(output),
        guard: None,
        span,
    }
}
