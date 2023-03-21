use nu_protocol::{
    ast::{Expr, Expression, MatchPattern, Pattern},
    engine::StateWorkingSet,
    Span, SyntaxShape, Type,
};

use crate::{
    lex,
    parser::{is_variable, parse_value},
    ParseError,
};

pub fn garbage(span: Span) -> MatchPattern {
    MatchPattern {
        pattern: Pattern::Garbage,
        span,
    }
}

pub fn parse_match_pattern(
    working_set: &mut StateWorkingSet,
    span: Span,
) -> (Expression, Option<ParseError>) {
    working_set.enter_scope();
    let (output, err) = parse_pattern(working_set, span);
    working_set.exit_scope();

    (
        Expression {
            expr: Expr::MatchPattern(Box::new(output)),
            span,
            ty: Type::Any,
            custom_completion: None,
        },
        err,
    )
}

pub fn parse_pattern(
    working_set: &mut StateWorkingSet,
    span: Span,
) -> (MatchPattern, Option<ParseError>) {
    let bytes = working_set.get_span_contents(span);

    if bytes.starts_with(b"$") {
        // Variable pattern
        parse_variable_pattern(working_set, span)
    } else if bytes.starts_with(b"{") {
        // Record pattern
        parse_record_pattern(working_set, span)
    } else {
        // Literal value
        let (value, error) = parse_value(working_set, span, &SyntaxShape::Any, &[]);
        (
            MatchPattern {
                pattern: Pattern::Value(value),
                span,
            },
            error,
        )
    }
}

pub fn parse_variable_pattern(
    working_set: &mut StateWorkingSet,
    span: Span,
) -> (MatchPattern, Option<ParseError>) {
    let bytes = working_set.get_span_contents(span);

    if is_variable(bytes) {
        if let Some(var_id) = working_set.find_variable(bytes) {
            (
                MatchPattern {
                    pattern: Pattern::Variable(var_id),
                    span,
                },
                None,
            )
        } else {
            let var_id = working_set.add_variable(bytes.to_vec(), span, Type::Any, true);

            (
                MatchPattern {
                    pattern: Pattern::Variable(var_id),
                    span,
                },
                None,
            )
        }
    } else {
        (
            garbage(span),
            Some(ParseError::Expected("valid variable name".into(), span)),
        )
    }
}

pub fn parse_record_pattern(
    working_set: &mut StateWorkingSet,
    span: Span,
) -> (MatchPattern, Option<ParseError>) {
    let bytes = working_set.get_span_contents(span);

    let mut error = None;
    let mut start = span.start;
    let mut end = span.end;

    if bytes.starts_with(b"{") {
        start += 1;
    } else {
        error = error.or_else(|| {
            Some(ParseError::Expected(
                "{".into(),
                Span::new(start, start + 1),
            ))
        });
    }

    if bytes.ends_with(b"}") {
        end -= 1;
    } else {
        error = error.or_else(|| Some(ParseError::Unclosed("}".into(), Span::new(end, end))));
    }

    let inner_span = Span::new(start, end);
    let source = working_set.get_span_contents(inner_span);

    let (tokens, err) = lex(source, start, &[b'\n', b'\r', b','], &[b':'], true);
    error = error.or(err);

    let mut output = vec![];
    let mut idx = 0;

    while idx < tokens.len() {
        let (field, err) = parse_pattern(working_set, tokens[idx].span);
        error = error.or(err);

        idx += 1;
        if idx == tokens.len() {
            return (
                garbage(span),
                Some(ParseError::Expected("record".into(), span)),
            );
        }
        let colon = working_set.get_span_contents(tokens[idx].span);
        idx += 1;
        if idx == tokens.len() || colon != b":" {
            //FIXME: need better error
            return (
                garbage(span),
                Some(ParseError::Expected("record".into(), span)),
            );
        }
        let (value, err) = parse_pattern(working_set, tokens[idx].span);
        error = error.or(err);
        idx += 1;

        output.push((field, value));
    }

    (
        MatchPattern {
            pattern: Pattern::Record(output),
            span,
        },
        error,
    )
}
