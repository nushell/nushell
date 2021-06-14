use std::{path::Path, sync::Arc};

use bigdecimal::BigDecimal;
use indexmap::IndexMap;
use log::trace;
use nu_errors::{ArgumentError, ParseError};
use nu_protocol::hir::{
    self, Binary, Block, ClassifiedCommand, Expression, ExternalRedirection, Flag, FlagKind, Group,
    InternalCommand, Member, NamedArguments, Operator, Pipeline, RangeOperator, SpannedExpression,
    Unit,
};
use nu_protocol::{NamedType, PositionalType, Signature, SyntaxShape, UnspannedPathMember};
use nu_source::{HasSpan, Span, Spanned, SpannedItem};
use num_bigint::BigInt;

use crate::{
    lex::lexer::{lex, parse_block},
    ParserScope,
};
use crate::{
    lex::{
        lexer::Token,
        tokens::{LiteBlock, LiteCommand, LitePipeline, TokenContents},
    },
    parse::def::lex_split_baseline_tokens_on,
};
use crate::{parse::def::parse_parameter, path::expand_path};

use self::{
    def::{parse_definition, parse_definition_prototype},
    util::trim_quotes,
    util::verify_and_strip,
};

mod def;
mod util;

pub use self::util::garbage;

/// Parses a simple column path, one without a variable (implied or explicit) at the head
pub fn parse_simple_column_path(
    lite_arg: &Spanned<String>,
) -> (SpannedExpression, Option<ParseError>) {
    let mut delimiter = '.';
    let mut inside_delimiter = false;
    let mut output = vec![];
    let mut current_part = String::new();
    let mut start_index = 0;
    let mut last_index = 0;

    for (idx, c) in lite_arg.item.char_indices() {
        last_index = idx;
        if inside_delimiter {
            if c == delimiter {
                inside_delimiter = false;
            }
        } else if c == '\'' || c == '"' {
            inside_delimiter = true;
            delimiter = c;
        } else if c == '.' {
            let part_span = Span::new(
                lite_arg.span.start() + start_index,
                lite_arg.span.start() + idx,
            );

            if let Ok(row_number) = current_part.parse::<i64>() {
                output.push(Member::Int(row_number, part_span));
            } else {
                let trimmed = trim_quotes(&current_part);
                output.push(Member::Bare(trimmed.clone().spanned(part_span)));
            }
            current_part.clear();
            // Note: I believe this is safe because of the delimiter we're using,
            // but if we get fancy with Unicode we'll need to change this.
            start_index = idx + '.'.len_utf8();
            continue;
        }
        current_part.push(c);
    }

    if !current_part.is_empty() {
        let part_span = Span::new(
            lite_arg.span.start() + start_index,
            lite_arg.span.start() + last_index + 1,
        );
        if let Ok(row_number) = current_part.parse::<i64>() {
            output.push(Member::Int(row_number, part_span));
        } else {
            let current_part = trim_quotes(&current_part);
            output.push(Member::Bare(current_part.spanned(part_span)));
        }
    }

    (
        SpannedExpression::new(Expression::simple_column_path(output), lite_arg.span),
        None,
    )
}

/// Parses a column path, adding in the preceding reference to $it if it's elided
pub fn parse_full_column_path(
    lite_arg: &Spanned<String>,
    scope: &dyn ParserScope,
) -> (SpannedExpression, Option<ParseError>) {
    let mut inside_delimiter = vec![];
    let mut output = vec![];
    let mut current_part = String::new();
    let mut start_index = 0;
    let mut last_index = 0;
    let mut error = None;

    let mut head = None;

    for (idx, c) in lite_arg.item.char_indices() {
        last_index = idx;
        if c == '(' {
            inside_delimiter.push(')');
        } else if let Some(delimiter) = inside_delimiter.last() {
            if c == *delimiter {
                inside_delimiter.pop();
            }
        } else if c == '\'' || c == '"' {
            inside_delimiter.push(c);
        } else if c == '.' {
            let part_span = Span::new(
                lite_arg.span.start() + start_index,
                lite_arg.span.start() + idx,
            );

            if head.is_none() && current_part.starts_with('(') && current_part.ends_with(')') {
                let (invoc, err) =
                    parse_subexpression(&current_part.clone().spanned(part_span), scope);
                if error.is_none() {
                    error = err;
                }
                head = Some(invoc.expr);
            } else if head.is_none() && current_part.starts_with('$') {
                // We have the variable head
                head = Some(Expression::variable(current_part.clone(), part_span))
            } else if let Ok(row_number) = current_part.parse::<i64>() {
                output.push(UnspannedPathMember::Int(row_number).into_path_member(part_span));
            } else {
                let current_part = trim_quotes(&current_part);
                output.push(
                    UnspannedPathMember::String(current_part.clone()).into_path_member(part_span),
                );
            }
            current_part.clear();
            // Note: I believe this is safe because of the delimiter we're using,
            // but if we get fancy with Unicode we'll need to change this.
            start_index = idx + '.'.len_utf8();
            continue;
        }
        current_part.push(c);
    }

    if !current_part.is_empty() {
        let part_span = Span::new(
            lite_arg.span.start() + start_index,
            lite_arg.span.start() + last_index + 1,
        );

        if head.is_none() {
            if current_part.starts_with('(') && current_part.ends_with(')') {
                let (invoc, err) = parse_subexpression(&current_part.spanned(part_span), scope);
                if error.is_none() {
                    error = err;
                }
                head = Some(invoc.expr);
            } else if current_part.starts_with('$') {
                head = Some(Expression::variable(current_part, lite_arg.span));
            } else if let Ok(row_number) = current_part.parse::<i64>() {
                output.push(UnspannedPathMember::Int(row_number).into_path_member(part_span));
            } else {
                let current_part = trim_quotes(&current_part);
                output.push(UnspannedPathMember::String(current_part).into_path_member(part_span));
            }
        } else if let Ok(row_number) = current_part.parse::<i64>() {
            output.push(UnspannedPathMember::Int(row_number).into_path_member(part_span));
        } else {
            let current_part = trim_quotes(&current_part);
            output.push(UnspannedPathMember::String(current_part).into_path_member(part_span));
        }
    }

    if let Some(head) = head {
        (
            SpannedExpression::new(
                Expression::path(SpannedExpression::new(head, lite_arg.span), output),
                lite_arg.span,
            ),
            error,
        )
    } else {
        (
            SpannedExpression::new(
                Expression::path(
                    SpannedExpression::new(
                        Expression::variable("$it".into(), lite_arg.span),
                        lite_arg.span,
                    ),
                    output,
                ),
                lite_arg.span,
            ),
            error,
        )
    }
}

/// Parse a numeric range
fn parse_range(
    lite_arg: &Spanned<String>,
    scope: &dyn ParserScope,
) -> (SpannedExpression, Option<ParseError>) {
    let lite_arg_span_start = lite_arg.span.start();
    let lite_arg_len = lite_arg.item.len();
    let (dotdot_pos, operator_str, operator) = if let Some(pos) = lite_arg.item.find("..<") {
        (pos, "..<", RangeOperator::RightExclusive)
    } else if let Some(pos) = lite_arg.item.find("..") {
        (pos, "..", RangeOperator::Inclusive)
    } else {
        return (
            garbage(lite_arg.span),
            Some(ParseError::mismatch("range", lite_arg.clone())),
        );
    };

    if lite_arg.item[0..dotdot_pos].is_empty()
        && lite_arg.item[(dotdot_pos + operator_str.len())..].is_empty()
    {
        return (
            garbage(lite_arg.span),
            Some(ParseError::mismatch("range", lite_arg.clone())),
        );
    }

    let numbers: Vec<_> = lite_arg.item.split(operator_str).collect();

    if numbers.len() != 2 {
        return (
            garbage(lite_arg.span),
            Some(ParseError::mismatch("range", lite_arg.clone())),
        );
    }

    let right_number_offset = operator_str.len();

    let lhs = numbers[0].to_string().spanned(Span::new(
        lite_arg_span_start,
        lite_arg_span_start + dotdot_pos,
    ));
    let rhs = numbers[1].to_string().spanned(Span::new(
        lite_arg_span_start + dotdot_pos + right_number_offset,
        lite_arg_span_start + lite_arg_len,
    ));

    let left_hand_open = dotdot_pos == 0;
    let right_hand_open = dotdot_pos == lite_arg_len - right_number_offset;

    let left = if left_hand_open {
        None
    } else if let (left, None) = parse_arg(SyntaxShape::Number, scope, &lhs) {
        Some(left)
    } else {
        return (
            garbage(lite_arg.span),
            Some(ParseError::mismatch("range", lhs)),
        );
    };

    let right = if right_hand_open {
        None
    } else if let (right, None) = parse_arg(SyntaxShape::Number, scope, &rhs) {
        Some(right)
    } else {
        return (
            garbage(lite_arg.span),
            Some(ParseError::mismatch("range", rhs)),
        );
    };

    (
        SpannedExpression::new(
            Expression::range(
                left,
                operator.spanned(Span::new(
                    lite_arg_span_start + dotdot_pos,
                    lite_arg_span_start + dotdot_pos + right_number_offset,
                )),
                right,
            ),
            lite_arg.span,
        ),
        None,
    )
}

/// Parse any allowed operator, including word-based operators
fn parse_operator(lite_arg: &Spanned<String>) -> (SpannedExpression, Option<ParseError>) {
    let operator = match &lite_arg.item[..] {
        "==" => Operator::Equal,
        "!=" => Operator::NotEqual,
        "<" => Operator::LessThan,
        "<=" => Operator::LessThanOrEqual,
        ">" => Operator::GreaterThan,
        ">=" => Operator::GreaterThanOrEqual,
        "=~" => Operator::Contains,
        "!~" => Operator::NotContains,
        "+" => Operator::Plus,
        "-" => Operator::Minus,
        "*" => Operator::Multiply,
        "/" => Operator::Divide,
        "in" => Operator::In,
        "not-in" => Operator::NotIn,
        "mod" => Operator::Modulo,
        "&&" => Operator::And,
        "||" => Operator::Or,
        "**" => Operator::Pow,
        _ => {
            return (
                garbage(lite_arg.span),
                Some(ParseError::mismatch("operator", lite_arg.clone())),
            );
        }
    };

    (
        SpannedExpression::new(Expression::operator(operator), lite_arg.span),
        None,
    )
}

/// Parse a duration type, eg '10day'
fn parse_duration(lite_arg: &Spanned<String>) -> (SpannedExpression, Option<ParseError>) {
    fn parse_decimal_str_to_number(decimal: &str) -> Option<i64> {
        let string_to_parse = format!("0.{}", decimal);
        if let Ok(x) = string_to_parse.parse::<f64>() {
            return Some((1_f64 / x) as i64);
        }
        None
    }
    let unit_groups = [
        (Unit::Nanosecond, "NS", None),
        (Unit::Microsecond, "US", Some((Unit::Nanosecond, 1000))),
        (Unit::Millisecond, "MS", Some((Unit::Microsecond, 1000))),
        (Unit::Second, "SEC", Some((Unit::Millisecond, 1000))),
        (Unit::Minute, "MIN", Some((Unit::Second, 60))),
        (Unit::Hour, "HR", Some((Unit::Minute, 60))),
        (Unit::Day, "DAY", Some((Unit::Minute, 1440))),
        (Unit::Week, "WK", Some((Unit::Day, 7))),
    ];
    if let Some(unit) = unit_groups
        .iter()
        .find(|&x| lite_arg.to_uppercase().ends_with(x.1))
    {
        let mut lhs = lite_arg.item.clone();
        for _ in 0..unit.1.len() {
            lhs.pop();
        }

        let input: Vec<&str> = lhs.split('.').collect();
        let (value, unit_to_use) = match &input[..] {
            [number_str] => (number_str.parse::<i64>().ok(), unit.0),
            [number_str, decimal_part_str] => match unit.2 {
                Some(unit_to_convert_to) => match (
                    number_str.parse::<i64>(),
                    parse_decimal_str_to_number(decimal_part_str),
                ) {
                    (Ok(number), Some(decimal_part)) => (
                        Some(
                            (number * unit_to_convert_to.1) + (unit_to_convert_to.1 / decimal_part),
                        ),
                        unit_to_convert_to.0,
                    ),
                    _ => (None, unit.0),
                },
                None => (None, unit.0),
            },
            _ => (None, unit.0),
        };

        if let Some(x) = value {
            let lhs_span = Span::new(lite_arg.span.start(), lite_arg.span.start() + lhs.len());
            let unit_span = Span::new(lite_arg.span.start() + lhs.len(), lite_arg.span.end());
            return (
                SpannedExpression::new(
                    Expression::unit(x.spanned(lhs_span), unit_to_use.spanned(unit_span)),
                    lite_arg.span,
                ),
                None,
            );
        }
    }

    (
        garbage(lite_arg.span),
        Some(ParseError::mismatch("duration", lite_arg.clone())),
    )
}

/// Parse a unit type, eg '10kb'
fn parse_filesize(lite_arg: &Spanned<String>) -> (SpannedExpression, Option<ParseError>) {
    fn parse_decimal_str_to_number(decimal: &str) -> Option<i64> {
        let string_to_parse = format!("0.{}", decimal);
        if let Ok(x) = string_to_parse.parse::<f64>() {
            return Some((1_f64 / x) as i64);
        }
        None
    }
    let unit_groups = [
        (Unit::Kilobyte, "KB", Some((Unit::Byte, 1000))),
        (Unit::Megabyte, "MB", Some((Unit::Kilobyte, 1000))),
        (Unit::Gigabyte, "GB", Some((Unit::Megabyte, 1000))),
        (Unit::Terabyte, "TB", Some((Unit::Gigabyte, 1000))),
        (Unit::Petabyte, "PB", Some((Unit::Terabyte, 1000))),
        (Unit::Kibibyte, "KIB", Some((Unit::Byte, 1024))),
        (Unit::Mebibyte, "MIB", Some((Unit::Kibibyte, 1024))),
        (Unit::Gibibyte, "GIB", Some((Unit::Mebibyte, 1024))),
        (Unit::Tebibyte, "TIB", Some((Unit::Gibibyte, 1024))),
        (Unit::Pebibyte, "PIB", Some((Unit::Tebibyte, 1024))),
        (Unit::Byte, "B", None),
    ];
    if let Some(unit) = unit_groups
        .iter()
        .find(|&x| lite_arg.to_uppercase().ends_with(x.1))
    {
        let mut lhs = lite_arg.item.clone();
        for _ in 0..unit.1.len() {
            lhs.pop();
        }

        let input: Vec<&str> = lhs.split('.').collect();
        let (value, unit_to_use) = match &input[..] {
            [number_str] => (number_str.parse::<i64>().ok(), unit.0),
            [number_str, decimal_part_str] => match unit.2 {
                Some(unit_to_convert_to) => match (
                    number_str.parse::<i64>(),
                    parse_decimal_str_to_number(decimal_part_str),
                ) {
                    (Ok(number), Some(decimal_part)) => (
                        Some(
                            (number * unit_to_convert_to.1) + (unit_to_convert_to.1 / decimal_part),
                        ),
                        unit_to_convert_to.0,
                    ),
                    _ => (None, unit.0),
                },
                None => (None, unit.0),
            },
            _ => (None, unit.0),
        };

        if let Some(x) = value {
            let lhs_span = Span::new(lite_arg.span.start(), lite_arg.span.start() + lhs.len());
            let unit_span = Span::new(lite_arg.span.start() + lhs.len(), lite_arg.span.end());
            return (
                SpannedExpression::new(
                    Expression::unit(x.spanned(lhs_span), unit_to_use.spanned(unit_span)),
                    lite_arg.span,
                ),
                None,
            );
        }
    }

    (
        garbage(lite_arg.span),
        Some(ParseError::mismatch("unit", lite_arg.clone())),
    )
}

fn parse_subexpression(
    lite_arg: &Spanned<String>,
    scope: &dyn ParserScope,
) -> (SpannedExpression, Option<ParseError>) {
    let mut error = None;

    let string: String = lite_arg
        .item
        .chars()
        .skip(1)
        .take(lite_arg.item.chars().count() - 2)
        .collect();

    // We haven't done much with the inner string, so let's go ahead and work with it
    let (tokens, err) = lex(&string, lite_arg.span.start() + 1);
    if error.is_none() {
        error = err;
    };
    let (lite_block, err) = parse_block(tokens);
    if error.is_none() {
        error = err;
    };

    scope.enter_scope();
    let (classified_block, err) = classify_block(&lite_block, scope);
    if error.is_none() {
        error = err;
    };
    scope.exit_scope();

    (
        SpannedExpression::new(Expression::Subexpression(classified_block), lite_arg.span),
        error,
    )
}

fn parse_variable(
    lite_arg: &Spanned<String>,
    scope: &dyn ParserScope,
) -> (SpannedExpression, Option<ParseError>) {
    if lite_arg.item == "$it" {
        trace!("parsing $it");
        parse_full_column_path(lite_arg, scope)
    } else {
        (
            SpannedExpression::new(
                Expression::variable(lite_arg.item.clone(), lite_arg.span),
                lite_arg.span,
            ),
            None,
        )
    }
}
/// Parses the given lite_arg starting with dollar returning
/// a expression starting with $
/// Currently either Variable, String interpolation, FullColumnPath
fn parse_dollar_expr(
    lite_arg: &Spanned<String>,
    scope: &dyn ParserScope,
) -> (SpannedExpression, Option<ParseError>) {
    trace!("Parsing dollar expression: {:?}", lite_arg.item);
    if (lite_arg.item.starts_with("$\"") && lite_arg.item.len() > 1 && lite_arg.item.ends_with('"'))
        || (lite_arg.item.starts_with("$'")
            && lite_arg.item.len() > 1
            && lite_arg.item.ends_with('\''))
    {
        // This is an interpolated string
        parse_interpolated_string(lite_arg, scope)
    } else if let (expr, None) = parse_range(lite_arg, scope) {
        (expr, None)
    } else if let (expr, None) = parse_full_column_path(lite_arg, scope) {
        (expr, None)
    } else {
        parse_variable(lite_arg, scope)
    }
}

#[derive(Debug)]
enum FormatCommand {
    Text(Spanned<String>),
    Column(Spanned<String>),
}

fn format(input: &str, start: usize) -> (Vec<FormatCommand>, Option<ParseError>) {
    let original_start = start;
    let mut output = vec![];
    let mut error = None;

    let mut loop_input = input.chars().peekable();
    let mut start = start;
    let mut end = start;
    loop {
        let mut before = String::new();

        loop {
            end += 1;
            if let Some(c) = loop_input.next() {
                if c == '(' {
                    break;
                }
                before.push(c);
            } else {
                break;
            }
        }

        if !before.is_empty() {
            output.push(FormatCommand::Text(
                before.to_string().spanned(Span::new(start, end - 1)),
            ));
        }
        // Look for column as we're now at one
        let mut column = String::new();
        start = end;

        let mut found_end = false;
        let mut delimiter_stack = vec![')'];

        for c in &mut loop_input {
            end += 1;
            if let Some('\'') = delimiter_stack.last() {
                if c == '\'' {
                    delimiter_stack.pop();
                }
            } else if let Some('"') = delimiter_stack.last() {
                if c == '"' {
                    delimiter_stack.pop();
                }
            } else if c == '\'' {
                delimiter_stack.push('\'');
            } else if c == '"' {
                delimiter_stack.push('"');
            } else if c == '(' {
                delimiter_stack.push(')');
            } else if c == ')' {
                if let Some(')') = delimiter_stack.last() {
                    delimiter_stack.pop();
                }
                if delimiter_stack.is_empty() {
                    found_end = true;
                    break;
                }
            }
            column.push(c);
        }

        if !column.is_empty() {
            output.push(FormatCommand::Column(
                column.to_string().spanned(Span::new(start, end)),
            ));
        }

        if column.is_empty() {
            break;
        }

        if !found_end {
            error = Some(ParseError::argument_error(
                input.spanned(Span::new(original_start, end)),
                ArgumentError::MissingValueForName("unclosed ()".to_string()),
            ));
        }

        start = end;
    }

    (output, error)
}

/// Parses an interpolated string, one that has expressions inside of it
fn parse_interpolated_string(
    lite_arg: &Spanned<String>,
    scope: &dyn ParserScope,
) -> (SpannedExpression, Option<ParseError>) {
    trace!("Parse_interpolated_string");
    let string_len = lite_arg.item.len();
    let inner_string = lite_arg
        .item
        .chars()
        .skip(2)
        .take(string_len - 3)
        .collect::<String>();
    let mut error = None;

    let (format_result, err) = format(&inner_string, lite_arg.span.start() + 2);

    if error.is_none() {
        error = err;
    }

    let mut output = vec![];

    for f in format_result {
        match f {
            FormatCommand::Text(t) => {
                output.push(SpannedExpression {
                    expr: Expression::Literal(hir::Literal::String(t.item)),
                    span: t.span,
                });
            }
            FormatCommand::Column(c) => {
                let result = parse(&c, c.span.start(), scope);
                match result {
                    (classified_block, None) => {
                        output.push(SpannedExpression {
                            expr: Expression::Subexpression(classified_block),
                            span: c.span,
                        });
                    }
                    (_, Some(err)) => {
                        return (garbage(c.span), Some(err));
                    }
                }
            }
        }
    }

    let pipelines = vec![Pipeline {
        span: lite_arg.span,
        list: vec![ClassifiedCommand::Internal(InternalCommand {
            name: "build-string".to_owned(),
            name_span: lite_arg.span,
            args: hir::Call {
                head: Box::new(SpannedExpression {
                    expr: Expression::Synthetic(hir::Synthetic::String("build-string".to_owned())),
                    span: lite_arg.span,
                }),
                external_redirection: ExternalRedirection::Stdout,
                named: None,
                positional: Some(output),
                span: lite_arg.span,
            },
        })],
    }];

    let group = Group::new(pipelines, lite_arg.span);

    let call = SpannedExpression {
        expr: Expression::Subexpression(Arc::new(Block::new(
            Signature::new("<subexpression>"),
            vec![group],
            IndexMap::new(),
            lite_arg.span,
        ))),
        span: lite_arg.span,
    };

    (call, error)
}

/// Parses the given argument using the shape as a guide for how to correctly parse the argument
fn parse_external_arg(
    lite_arg: &Spanned<String>,
    scope: &dyn ParserScope,
) -> (SpannedExpression, Option<ParseError>) {
    if lite_arg.item.starts_with('$') {
        parse_dollar_expr(lite_arg, scope)
    } else if lite_arg.item.starts_with('(') {
        parse_subexpression(lite_arg, scope)
    } else {
        (
            SpannedExpression::new(Expression::string(lite_arg.item.clone()), lite_arg.span),
            None,
        )
    }
}

fn parse_list(
    lite_block: &LiteBlock,
    scope: &dyn ParserScope,
) -> (Vec<SpannedExpression>, Option<ParseError>) {
    let mut error = None;

    if lite_block.block.is_empty() {
        return (vec![], None);
    }
    let lite_pipeline = &lite_block.block[0];
    let mut output = vec![];
    for lite_pipeline in &lite_pipeline.pipelines {
        for lite_inner in &lite_pipeline.commands {
            for part in &lite_inner.parts {
                let item = if part.ends_with(',') {
                    let mut str: String = part.item.clone();
                    str.pop();
                    str.spanned(Span::new(part.span.start(), part.span.end() - 1))
                } else {
                    part.clone()
                };
                let (part, err) = parse_arg(SyntaxShape::Any, scope, &item);
                output.push(part);

                if error.is_none() {
                    error = err;
                }
            }
        }
    }

    (output, error)
}

fn parse_table(
    lite_block: &LiteBlock,
    scope: &dyn ParserScope,
    span: Span,
) -> (SpannedExpression, Option<ParseError>) {
    let mut error = None;
    let mut output = vec![];

    // Header
    let lite_group = &lite_block.block[0];
    let lite_pipeline = &lite_group.pipelines[0];
    let lite_inner = &lite_pipeline.commands[0];

    let (string, err) = verify_and_strip(&lite_inner.parts[0], '[', ']');
    if error.is_none() {
        error = err;
    }

    let (tokens, err) = lex(&string, lite_inner.parts[0].span.start() + 1);
    if err.is_some() {
        return (garbage(lite_inner.span()), err);
    }

    let (lite_header, err) = parse_block(tokens);
    if err.is_some() {
        return (garbage(lite_inner.span()), err);
    }

    let (headers, err) = parse_list(&lite_header, scope);
    if error.is_none() {
        error = err;
    }

    // Cells
    let lite_rows = &lite_group.pipelines[1];
    let lite_cells = &lite_rows.commands[0];

    for arg in &lite_cells.parts {
        let (string, err) = verify_and_strip(arg, '[', ']');
        if error.is_none() {
            error = err;
        }
        let (tokens, err) = lex(&string, arg.span.start() + 1);
        if err.is_some() {
            return (garbage(arg.span), err);
        }
        let (lite_cell, err) = parse_block(tokens);
        if err.is_some() {
            return (garbage(arg.span), err);
        }
        let (inner_cell, err) = parse_list(&lite_cell, scope);
        if error.is_none() {
            error = err;
        }
        output.push(inner_cell);
    }

    (
        SpannedExpression::new(Expression::Table(headers, output), span),
        error,
    )
}

fn parse_int(lite_arg: &Spanned<String>) -> (SpannedExpression, Option<ParseError>) {
    if lite_arg.item.starts_with("0x") {
        if let Ok(v) = i64::from_str_radix(&lite_arg.item[2..], 16) {
            (
                SpannedExpression::new(Expression::integer(v), lite_arg.span),
                None,
            )
        } else {
            (
                garbage(lite_arg.span),
                Some(ParseError::mismatch("int", lite_arg.clone())),
            )
        }
    } else if lite_arg.item.starts_with("0b") {
        if let Ok(v) = i64::from_str_radix(&lite_arg.item[2..], 2) {
            (
                SpannedExpression::new(Expression::integer(v), lite_arg.span),
                None,
            )
        } else {
            (
                garbage(lite_arg.span),
                Some(ParseError::mismatch("int", lite_arg.clone())),
            )
        }
    } else if lite_arg.item.starts_with("0o") {
        if let Ok(v) = i64::from_str_radix(&lite_arg.item[2..], 8) {
            (
                SpannedExpression::new(Expression::integer(v), lite_arg.span),
                None,
            )
        } else {
            (
                garbage(lite_arg.span),
                Some(ParseError::mismatch("int", lite_arg.clone())),
            )
        }
    } else if let Ok(x) = lite_arg.item.parse::<i64>() {
        (
            SpannedExpression::new(Expression::integer(x), lite_arg.span),
            None,
        )
    } else {
        (
            garbage(lite_arg.span),
            Some(ParseError::mismatch("int", lite_arg.clone())),
        )
    }
}

/// Parses the given argument using the shape as a guide for how to correctly parse the argument
fn parse_arg(
    expected_type: SyntaxShape,
    scope: &dyn ParserScope,
    lite_arg: &Spanned<String>,
) -> (SpannedExpression, Option<ParseError>) {
    if lite_arg.item.starts_with('$') {
        return parse_dollar_expr(lite_arg, scope);
    }

    // before anything else, try to see if this is a number in paranthesis
    if lite_arg.item.starts_with('(') {
        return parse_full_column_path(lite_arg, scope);
    }

    match expected_type {
        SyntaxShape::Number => {
            if let (x, None) = parse_int(lite_arg) {
                (x, None)
            } else if let Ok(x) = lite_arg.item.parse::<BigInt>() {
                (
                    SpannedExpression::new(Expression::big_integer(x), lite_arg.span),
                    None,
                )
            } else if let Ok(x) = lite_arg.item.parse::<BigDecimal>() {
                (
                    SpannedExpression::new(Expression::decimal(x), lite_arg.span),
                    None,
                )
            } else {
                (
                    garbage(lite_arg.span),
                    Some(ParseError::mismatch("number", lite_arg.clone())),
                )
            }
        }
        SyntaxShape::Int => {
            if let Ok(x) = lite_arg.item.parse::<i64>() {
                (
                    SpannedExpression::new(Expression::integer(x), lite_arg.span),
                    None,
                )
            } else if let Ok(x) = lite_arg.item.parse::<BigInt>() {
                (
                    SpannedExpression::new(Expression::big_integer(x), lite_arg.span),
                    None,
                )
            } else {
                (
                    garbage(lite_arg.span),
                    Some(ParseError::mismatch("int", lite_arg.clone())),
                )
            }
        }
        SyntaxShape::String => {
            let trimmed = trim_quotes(&lite_arg.item);
            (
                SpannedExpression::new(Expression::string(trimmed), lite_arg.span),
                None,
            )
        }
        SyntaxShape::GlobPattern => {
            let trimmed = trim_quotes(&lite_arg.item);
            let expanded = expand_path(&trimmed).to_string();
            (
                SpannedExpression::new(Expression::glob_pattern(expanded), lite_arg.span),
                None,
            )
        }

        SyntaxShape::Range => parse_range(lite_arg, scope),
        SyntaxShape::Operator => (
            garbage(lite_arg.span),
            Some(ParseError::mismatch("operator", lite_arg.clone())),
        ),
        SyntaxShape::Filesize => parse_filesize(lite_arg),
        SyntaxShape::Duration => parse_duration(lite_arg),
        SyntaxShape::FilePath => {
            let trimmed = trim_quotes(&lite_arg.item);
            let expanded = expand_path(&trimmed).to_string();
            let path = Path::new(&expanded);
            (
                SpannedExpression::new(Expression::FilePath(path.to_path_buf()), lite_arg.span),
                None,
            )
        }
        SyntaxShape::ColumnPath => parse_simple_column_path(lite_arg),
        SyntaxShape::FullColumnPath => parse_full_column_path(lite_arg, scope),
        SyntaxShape::Any => {
            let shapes = vec![
                SyntaxShape::Int,
                SyntaxShape::Number,
                SyntaxShape::Range,
                SyntaxShape::Filesize,
                SyntaxShape::Duration,
                SyntaxShape::Block,
                SyntaxShape::Table,
                SyntaxShape::String,
            ];
            for shape in shapes.iter() {
                if let (s, None) = parse_arg(*shape, scope, lite_arg) {
                    return (s, None);
                }
            }
            (
                garbage(lite_arg.span),
                Some(ParseError::mismatch("any shape", lite_arg.clone())),
            )
        }
        SyntaxShape::Table => {
            let mut chars = lite_arg.item.chars();

            match (chars.next(), chars.next_back()) {
                (Some('['), Some(']')) => {
                    // We have a literal row
                    let string: String = chars.collect();

                    // We haven't done much with the inner string, so let's go ahead and work with it
                    let (tokens, err) = lex(&string, lite_arg.span.start() + 1);
                    if err.is_some() {
                        return (garbage(lite_arg.span), err);
                    }

                    let (lite_block, err) = parse_block(tokens);
                    if err.is_some() {
                        return (garbage(lite_arg.span), err);
                    }

                    let lite_groups = &lite_block.block;

                    if lite_groups.is_empty() {
                        return (
                            SpannedExpression::new(Expression::List(vec![]), lite_arg.span),
                            None,
                        );
                    }
                    if lite_groups[0].pipelines.len() == 1 {
                        let (items, err) = parse_list(&lite_block, scope);
                        (
                            SpannedExpression::new(Expression::List(items), lite_arg.span),
                            err,
                        )
                    } else if lite_groups[0].pipelines.len() == 2 {
                        parse_table(&lite_block, scope, lite_arg.span)
                    } else {
                        (
                            garbage(lite_arg.span),
                            Some(ParseError::mismatch(
                                "list or table",
                                "unknown".to_string().spanned(lite_arg.span),
                            )),
                        )
                    }
                }
                _ => (
                    garbage(lite_arg.span),
                    Some(ParseError::mismatch("table", lite_arg.clone())),
                ),
            }
        }
        SyntaxShape::MathExpression => parse_arg(SyntaxShape::Any, scope, lite_arg),

        SyntaxShape::Block | SyntaxShape::RowCondition => {
            // Blocks have one of two forms: the literal block and the implied block
            // To parse a literal block, we need to detect that what we have is itself a block
            let mut chars: Vec<_> = lite_arg.item.chars().collect();

            match chars.first() {
                Some('{') => {
                    let mut error = None;

                    if let Some('}') = chars.last() {
                        chars = chars[1..(chars.len() - 1)].to_vec();
                    } else {
                        chars = chars[1..].to_vec();
                        error = Some(ParseError::unclosed(
                            "}".into(),
                            Span::new(lite_arg.span.end(), lite_arg.span.end()),
                        ));
                    }
                    // We have a literal block
                    let string: String = chars.into_iter().collect();

                    // We haven't done much with the inner string, so let's go ahead and work with it
                    let (mut tokens, err) = lex(&string, lite_arg.span.start() + 1);
                    if error.is_none() {
                        error = err;
                    }

                    // Check to see if we have parameters
                    let params = if matches!(
                        tokens.first(),
                        Some(Token {
                            contents: TokenContents::Pipe,
                            ..
                        })
                    ) {
                        // We've found a parameter list
                        let mut param_tokens = vec![];
                        let mut token_iter = tokens.into_iter().skip(1);
                        for token in &mut token_iter {
                            if matches!(
                                token,
                                Token {
                                    contents: TokenContents::Pipe,
                                    ..
                                }
                            ) {
                                break;
                            } else {
                                param_tokens.push(token);
                            }
                        }
                        let split_tokens =
                            lex_split_baseline_tokens_on(param_tokens, &[',', ':', '?']);

                        let mut i = 0;
                        let mut params = vec![];

                        while i < split_tokens.len() {
                            let (parameter, advance_by, error) =
                                parse_parameter(&split_tokens[i..], split_tokens[i].span);

                            if error.is_some() {
                                return (garbage(lite_arg.span), error);
                            }
                            i += advance_by;
                            params.push(parameter);
                        }

                        tokens = token_iter.collect();
                        if tokens.is_empty() {
                            return (
                                garbage(lite_arg.span),
                                Some(ParseError::mismatch(
                                    "block with parameters",
                                    lite_arg.clone(),
                                )),
                            );
                        }
                        params
                    } else {
                        vec![]
                    };

                    let (lite_block, err) = parse_block(tokens);
                    if error.is_none() {
                        error = err;
                    }

                    scope.enter_scope();
                    let (mut classified_block, err) = classify_block(&lite_block, scope);
                    if error.is_none() {
                        error = err;
                    }
                    scope.exit_scope();

                    if let Some(classified_block) = Arc::get_mut(&mut classified_block) {
                        classified_block.span = lite_arg.span;
                        if !params.is_empty() {
                            classified_block.params.positional.clear();
                            for param in params {
                                classified_block
                                    .params
                                    .positional
                                    .push((param.pos_type, param.desc.unwrap_or_default()));
                            }
                        }
                    }

                    (
                        SpannedExpression::new(Expression::Block(classified_block), lite_arg.span),
                        error,
                    )
                }
                _ => {
                    // We have an implied block, but we can't parse this here
                    // it needed to have been parsed up higher where we have control over more than one arg
                    (
                        garbage(lite_arg.span),
                        Some(ParseError::mismatch("block", lite_arg.clone())),
                    )
                }
            }
        }
    }
}

/// This is a bit of a "fix-up" of previously parsed areas. In cases where we're in shorthand mode (eg in the `where` command), we need
/// to use the original source to parse a column path. Without it, we'll lose a little too much information to parse it correctly. As we'll
/// only know we were on the left-hand side of an expression after we do the full math parse, we need to do this step after rather than during
/// the initial parse.
fn shorthand_reparse(
    left: SpannedExpression,
    orig_left: Option<Spanned<String>>,
    scope: &dyn ParserScope,
    shorthand_mode: bool,
) -> (SpannedExpression, Option<ParseError>) {
    // If we're in shorthand mode, we need to reparse the left-hand side if possible
    if shorthand_mode {
        if let Some(orig_left) = orig_left {
            parse_arg(SyntaxShape::FullColumnPath, scope, &orig_left)
        } else {
            (left, None)
        }
    } else {
        (left, None)
    }
}

fn parse_possibly_parenthesized(
    lite_arg: &Spanned<String>,
    scope: &dyn ParserScope,
) -> (
    (Option<Spanned<String>>, SpannedExpression),
    Option<ParseError>,
) {
    let (lhs, err) = parse_arg(SyntaxShape::Any, scope, lite_arg);
    ((Some(lite_arg.clone()), lhs), err)
}

/// Handle parsing math expressions, complete with working with the precedence of the operators
pub fn parse_math_expression(
    incoming_idx: usize,
    lite_args: &[Spanned<String>],
    scope: &dyn ParserScope,
    shorthand_mode: bool,
) -> (usize, SpannedExpression, Option<ParseError>) {
    // Precedence parsing is included
    // shorthand_mode means that the left-hand side of an expression can point to a column-path.
    // To make this possible, we parse as normal, but then go back and when we detect a
    // left-hand side, reparse that value if it's a string

    let mut idx = 0;
    let mut error = None;

    let mut working_exprs = vec![];
    let mut prec = vec![];

    let (lhs_working_expr, err) = parse_possibly_parenthesized(&lite_args[idx], scope);

    if error.is_none() {
        error = err;
    }
    working_exprs.push(lhs_working_expr);

    idx += 1;

    prec.push(0);

    while idx < lite_args.len() {
        let (op, err) = parse_operator(&lite_args[idx]);
        if error.is_none() {
            error = err;
        }
        idx += 1;

        if idx == lite_args.len() {
            if error.is_none() {
                error = Some(ParseError::argument_error(
                    lite_args[idx - 1].clone(),
                    ArgumentError::MissingMandatoryPositional("right hand side".into()),
                ));
            }
            working_exprs.push((None, garbage(op.span)));
            working_exprs.push((None, garbage(op.span)));
            prec.push(0);
            break;
        }

        trace!(
            "idx: {} working_exprs: {:#?} prec: {:?}",
            idx,
            working_exprs,
            prec
        );

        let (rhs_working_expr, err) = parse_possibly_parenthesized(&lite_args[idx], scope);

        if error.is_none() {
            error = err;
        }

        let next_prec = op.precedence();

        if !prec.is_empty() && next_prec > *prec.last().expect("this shouldn't happen") {
            prec.push(next_prec);
            working_exprs.push((None, op));
            working_exprs.push(rhs_working_expr);
            idx += 1;
            continue;
        }
        while !prec.is_empty()
                    && *prec.last().expect("This shouldn't happen") >= next_prec
                    && next_prec > 0 // Not garbage
                    && working_exprs.len() >= 3
        {
            // Pop 3 and create and expression, push and repeat
            trace!(
                "idx: {} working_exprs: {:#?} prec: {:?}",
                idx,
                working_exprs,
                prec
            );
            let (_, right) = working_exprs.pop().expect("This shouldn't be possible");
            let (_, op) = working_exprs.pop().expect("This shouldn't be possible");
            let (orig_left, left) = working_exprs.pop().expect("This shouldn't be possible");

            // If we're in shorthand mode, we need to reparse the left-hand side if possible
            let (left, err) = shorthand_reparse(left, orig_left, scope, shorthand_mode);
            if error.is_none() {
                error = err;
            }

            let span = Span::new(left.span.start(), right.span.end());
            working_exprs.push((
                None,
                SpannedExpression {
                    expr: Expression::Binary(Box::new(Binary { left, op, right })),
                    span,
                },
            ));
            prec.pop();
        }
        working_exprs.push((None, op));
        working_exprs.push(rhs_working_expr);
        prec.push(next_prec);

        idx += 1;
    }

    while working_exprs.len() >= 3 {
        // Pop 3 and create and expression, push and repeat
        let (_, right) = working_exprs.pop().expect("This shouldn't be possible");
        let (_, op) = working_exprs.pop().expect("This shouldn't be possible");
        let (orig_left, left) = working_exprs.pop().expect("This shouldn't be possible");

        let (left, err) = shorthand_reparse(left, orig_left, scope, shorthand_mode);
        if error.is_none() {
            error = err;
        }

        let span = Span::new(left.span.start(), right.span.end());
        working_exprs.push((
            None,
            SpannedExpression {
                expr: Expression::Binary(Box::new(Binary { left, op, right })),
                span,
            },
        ));
    }

    let (orig_left, left) = working_exprs.pop().expect("This shouldn't be possible");
    let (left, err) = shorthand_reparse(left, orig_left, scope, shorthand_mode);
    if error.is_none() {
        error = err;
    }

    (incoming_idx + idx, left, error)
}

/// Handles parsing the positional arguments as a batch
/// This allows us to check for times where multiple arguments are treated as one shape, as is the case with SyntaxShape::Math
fn parse_positional_argument(
    idx: usize,
    lite_cmd: &LiteCommand,
    positional_type: &PositionalType,
    remaining_positionals: usize,
    scope: &dyn ParserScope,
) -> (usize, SpannedExpression, Option<ParseError>) {
    let mut idx = idx;
    let mut error = None;
    let arg = match positional_type {
        PositionalType::Mandatory(_, SyntaxShape::MathExpression)
        | PositionalType::Optional(_, SyntaxShape::MathExpression) => {
            let end_idx = if (lite_cmd.parts.len() - 1) > remaining_positionals {
                lite_cmd.parts.len() - remaining_positionals
            } else {
                lite_cmd.parts.len()
            };

            let (new_idx, arg, err) =
                parse_math_expression(idx, &lite_cmd.parts[idx..end_idx], scope, false);

            let span = arg.span;
            let mut commands = hir::Pipeline::new(span);
            commands.push(ClassifiedCommand::Expr(Box::new(arg)));

            let block = hir::Block::new(
                Signature::new("<initializer>"),
                vec![Group::new(vec![commands], lite_cmd.span())],
                IndexMap::new(),
                span,
            );

            let arg = SpannedExpression::new(Expression::Block(Arc::new(block)), span);

            idx = new_idx - 1;
            if error.is_none() {
                error = err;
            }
            arg
        }
        PositionalType::Mandatory(_, SyntaxShape::RowCondition)
        | PositionalType::Optional(_, SyntaxShape::RowCondition) => {
            // A condition can take up multiple arguments, as we build the operation as <arg> <operator> <arg>
            // We need to do this here because in parse_arg, we have access to only one arg at a time

            if idx < lite_cmd.parts.len() {
                if lite_cmd.parts[idx].item.starts_with('{') {
                    // It's an explicit math expression, so parse it deeper in
                    let (arg, err) =
                        parse_arg(SyntaxShape::RowCondition, scope, &lite_cmd.parts[idx]);
                    if error.is_none() {
                        error = err;
                    }
                    arg
                } else {
                    let end_idx = if (lite_cmd.parts.len() - 1) > remaining_positionals {
                        lite_cmd.parts.len() - remaining_positionals
                    } else {
                        lite_cmd.parts.len()
                    };

                    let (new_idx, arg, err) =
                        parse_math_expression(idx, &lite_cmd.parts[idx..end_idx], scope, true);

                    let span = arg.span;
                    let mut commands = hir::Pipeline::new(span);
                    commands.push(ClassifiedCommand::Expr(Box::new(arg)));

                    let mut block = hir::Block::new(
                        Signature::new("<cond>"),
                        vec![Group::new(vec![commands], lite_cmd.span())],
                        IndexMap::new(),
                        span,
                    );

                    block.infer_params();

                    let arg = SpannedExpression::new(Expression::Block(Arc::new(block)), span);

                    idx = new_idx - 1;
                    if error.is_none() {
                        error = err;
                    }
                    arg
                }
            } else {
                if error.is_none() {
                    error = Some(ParseError::argument_error(
                        lite_cmd.parts[0].clone(),
                        ArgumentError::MissingMandatoryPositional("condition".into()),
                    ))
                }
                garbage(lite_cmd.span())
            }
        }
        PositionalType::Mandatory(_, shape) | PositionalType::Optional(_, shape) => {
            let (arg, err) = parse_arg(*shape, scope, &lite_cmd.parts[idx]);
            if error.is_none() {
                error = err;
            }
            arg
        }
    };

    (idx, arg, error)
}

/// Does a full parse of an internal command using the lite-ly parse command as a starting point
/// This main focus at this level is to understand what flags were passed in, what positional arguments were passed in, what rest arguments were passed in
/// and to ensure that the basic requirements in terms of number of each were met.
fn parse_internal_command(
    lite_cmd: &LiteCommand,
    scope: &dyn ParserScope,
    signature: &Signature,
    mut idx: usize,
) -> (InternalCommand, Option<ParseError>) {
    // This is a known internal command, so we need to work with the arguments and parse them according to the expected types

    let (name, name_span) = (
        lite_cmd.parts[0..(idx + 1)]
            .iter()
            .map(|x| x.item.clone())
            .collect::<Vec<String>>()
            .join(" "),
        Span::new(
            lite_cmd.parts[0].span.start(),
            lite_cmd.parts[idx].span.end(),
        ),
    );

    let mut internal_command = InternalCommand::new(name, name_span, lite_cmd.span());
    internal_command.args.set_initial_flags(signature);

    let mut current_positional = 0;
    let mut named = NamedArguments::new();
    let mut positional = vec![];
    let mut error = None;
    idx += 1; // Start where the arguments begin

    while idx < lite_cmd.parts.len() {
        if lite_cmd.parts[idx].item.starts_with('-') && lite_cmd.parts[idx].item.len() > 1 {
            let (named_types, err) = super::flag::get_flag_signature_spec(
                signature,
                &internal_command,
                &lite_cmd.parts[idx],
            );

            if err.is_none() {
                for (full_name, named_type) in &named_types {
                    match named_type {
                        NamedType::Mandatory(_, shape) | NamedType::Optional(_, shape) => {
                            if lite_cmd.parts[idx].item.contains('=') {
                                let mut offset = 0;

                                let value = lite_cmd.parts[idx]
                                    .item
                                    .chars()
                                    .skip_while(|prop| {
                                        offset += 1;
                                        *prop != '='
                                    })
                                    .nth(1);

                                offset = if value.is_none() { offset - 1 } else { offset };

                                let flag_value = Span::new(
                                    lite_cmd.parts[idx].span.start() + offset,
                                    lite_cmd.parts[idx].span.end(),
                                );
                                let value = lite_cmd.parts[idx].item[offset..]
                                    .to_string()
                                    .spanned(flag_value);
                                let (arg, err) = parse_arg(*shape, scope, &value);
                                named.insert_mandatory(
                                    full_name.clone(),
                                    lite_cmd.parts[idx].span,
                                    arg,
                                );

                                if error.is_none() {
                                    error = err;
                                }
                            } else if idx == lite_cmd.parts.len() {
                                // Oops, we're missing the argument to our named argument
                                if error.is_none() {
                                    error = Some(ParseError::argument_error(
                                        lite_cmd.parts[0].clone(),
                                        ArgumentError::MissingValueForName(format!("{:?}", shape)),
                                    ));
                                }
                            } else {
                                idx += 1;
                                if lite_cmd.parts.len() > idx {
                                    let (arg, err) = parse_arg(*shape, scope, &lite_cmd.parts[idx]);
                                    named.insert_mandatory(
                                        full_name.clone(),
                                        lite_cmd.parts[idx - 1].span,
                                        arg,
                                    );

                                    if error.is_none() {
                                        error = err;
                                    }
                                } else if error.is_none() {
                                    error = Some(ParseError::argument_error(
                                        lite_cmd.parts[0].clone(),
                                        ArgumentError::MissingValueForName(full_name.to_owned()),
                                    ));
                                }
                            }
                        }
                        NamedType::Switch(_) => {
                            named.insert_switch(
                                full_name.clone(),
                                Some(Flag::new(FlagKind::Longhand, lite_cmd.parts[idx].span)),
                            );
                        }
                    }
                }
            } else {
                positional.push(garbage(lite_cmd.parts[idx].span));

                if error.is_none() {
                    error = err;
                }
            }
        } else if signature.positional.len() > current_positional {
            let arg = {
                let (new_idx, expr, err) = parse_positional_argument(
                    idx,
                    lite_cmd,
                    &signature.positional[current_positional].0,
                    signature.positional.len() - current_positional - 1,
                    scope,
                );
                idx = new_idx;
                if error.is_none() {
                    error = err;
                }
                expr
            };

            positional.push(arg);
            current_positional += 1;
        } else if let Some((rest_type, _)) = &signature.rest_positional {
            let (arg, err) = parse_arg(*rest_type, scope, &lite_cmd.parts[idx]);
            if error.is_none() {
                error = err;
            }

            positional.push(arg);
            current_positional += 1;
        } else {
            positional.push(garbage(lite_cmd.parts[idx].span));

            if error.is_none() {
                error = Some(ParseError::argument_error(
                    lite_cmd.parts[0].clone(),
                    ArgumentError::UnexpectedArgument(lite_cmd.parts[idx].clone()),
                ));
            }
        }

        idx += 1;
    }

    // Count the required positional arguments and ensure these have been met
    let mut required_arg_count = 0;
    for positional_arg in &signature.positional {
        if let PositionalType::Mandatory(_, _) = positional_arg.0 {
            required_arg_count += 1;
        }
    }
    if positional.len() < required_arg_count && error.is_none() {
        // to make "command -h" work even if required arguments are missing
        if !named.named.contains_key("help") {
            let (_, name) = &signature.positional[positional.len()];
            error = Some(ParseError::argument_error(
                lite_cmd.parts[0].clone(),
                ArgumentError::MissingMandatoryPositional(name.to_owned()),
            ));
        }
    }

    if !named.is_empty() {
        internal_command.args.named = Some(named);
    }

    if !positional.is_empty() {
        internal_command.args.positional = Some(positional);
    }

    (internal_command, error)
}

fn parse_external_call(
    lite_cmd: &LiteCommand,
    end_of_pipeline: bool,
    scope: &dyn ParserScope,
) -> (Option<ClassifiedCommand>, Option<ParseError>) {
    let mut error = None;
    let name = lite_cmd.parts[0].clone().map(|v| {
        let trimmed = trim_quotes(&v);
        expand_path(&trimmed).to_string()
    });

    let mut args = vec![];

    let (name, err) = parse_arg(SyntaxShape::String, scope, &name);
    let name_span = name.span;
    if error.is_none() {
        error = err;
    }
    args.push(name);

    for lite_arg in &lite_cmd.parts[1..] {
        let (expr, err) = parse_external_arg(lite_arg, scope);
        if error.is_none() {
            error = err;
        }
        args.push(expr);
    }

    (
        Some(ClassifiedCommand::Internal(InternalCommand {
            name: "run_external".to_string(),
            name_span,
            args: hir::Call {
                head: Box::new(SpannedExpression {
                    expr: Expression::string("run_external".to_string()),
                    span: name_span,
                }),
                positional: Some(args),
                named: None,
                span: name_span,
                external_redirection: if end_of_pipeline {
                    ExternalRedirection::None
                } else {
                    ExternalRedirection::Stdout
                },
            },
        })),
        error,
    )
}

fn parse_value_call(
    call: LiteCommand,
    scope: &dyn ParserScope,
) -> (Option<ClassifiedCommand>, Option<ParseError>) {
    let mut err = None;

    let (head, error) = parse_arg(SyntaxShape::Block, scope, &call.parts[0]);
    let mut span = head.span;
    if err.is_none() {
        err = error;
    }

    let mut args = vec![];
    for arg in call.parts.iter().skip(1) {
        let (arg, error) = parse_arg(SyntaxShape::Any, scope, arg);
        if err.is_none() {
            err = error;
        }
        span = span.until(arg.span);
        args.push(arg);
    }

    (
        Some(ClassifiedCommand::Dynamic(hir::Call {
            head: Box::new(head),
            positional: Some(args),
            named: None,
            span,
            external_redirection: ExternalRedirection::None,
        })),
        err,
    )
}

fn expand_aliases_in_call(call: &mut LiteCommand, scope: &dyn ParserScope) {
    if let Some(name) = call.parts.get(0) {
        if let Some(mut expansion) = scope.get_alias(name) {
            // set the expansion's spans to point to the alias itself
            for item in expansion.iter_mut() {
                item.span = name.span;
            }

            // replace the alias with the expansion
            call.parts.remove(0);
            expansion.append(&mut call.parts);
            call.parts = expansion;
        }
    }
}

fn parse_call(
    mut lite_cmd: LiteCommand,
    end_of_pipeline: bool,
    scope: &dyn ParserScope,
) -> (Option<ClassifiedCommand>, Option<ParseError>) {
    expand_aliases_in_call(&mut lite_cmd, scope);

    let mut error = None;
    if lite_cmd.parts.is_empty() {
        return (None, None);
    } else if lite_cmd.parts[0].item.starts_with('^') {
        let mut name = lite_cmd.parts[0]
            .clone()
            .map(|v| v.chars().skip(1).collect::<String>());

        name.span = Span::new(name.span.start() + 1, name.span.end());

        // TODO this is the same as the `else` branch below, only the name differs. Find a way
        //      to share this functionality.
        let mut args = vec![];

        let (name, err) = parse_arg(SyntaxShape::String, scope, &name);
        let name_span = name.span;
        if error.is_none() {
            error = err;
        }
        args.push(name);

        for lite_arg in &lite_cmd.parts[1..] {
            let (expr, err) = parse_external_arg(lite_arg, scope);
            if error.is_none() {
                error = err;
            }
            args.push(expr);
        }

        return (
            Some(ClassifiedCommand::Internal(InternalCommand {
                name: "run_external".to_string(),
                name_span,
                args: hir::Call {
                    head: Box::new(SpannedExpression {
                        expr: Expression::string("run_external".to_string()),
                        span: name_span,
                    }),
                    positional: Some(args),
                    named: None,
                    span: name_span,
                    external_redirection: if end_of_pipeline {
                        ExternalRedirection::None
                    } else {
                        ExternalRedirection::Stdout
                    },
                },
            })),
            error,
        );
    } else if lite_cmd.parts[0].item.starts_with('{') {
        return parse_value_call(lite_cmd, scope);
    } else if lite_cmd.parts[0].item.starts_with('$')
        || lite_cmd.parts[0].item.starts_with('\"')
        || lite_cmd.parts[0].item.starts_with('\'')
        || (lite_cmd.parts[0].item.starts_with('-')
            && parse_arg(SyntaxShape::Number, scope, &lite_cmd.parts[0])
                .1
                .is_none())
        || (lite_cmd.parts[0].item.starts_with('-')
            && parse_arg(SyntaxShape::Range, scope, &lite_cmd.parts[0])
                .1
                .is_none())
        || lite_cmd.parts[0].item.starts_with('0')
        || lite_cmd.parts[0].item.starts_with('1')
        || lite_cmd.parts[0].item.starts_with('2')
        || lite_cmd.parts[0].item.starts_with('3')
        || lite_cmd.parts[0].item.starts_with('4')
        || lite_cmd.parts[0].item.starts_with('5')
        || lite_cmd.parts[0].item.starts_with('6')
        || lite_cmd.parts[0].item.starts_with('7')
        || lite_cmd.parts[0].item.starts_with('8')
        || lite_cmd.parts[0].item.starts_with('9')
        || lite_cmd.parts[0].item.starts_with('[')
        || lite_cmd.parts[0].item.starts_with('(')
    {
        let (_, expr, err) = parse_math_expression(0, &lite_cmd.parts[..], scope, false);
        error = error.or(err);
        return (Some(ClassifiedCommand::Expr(Box::new(expr))), error);
    } else if lite_cmd.parts.len() > 1 {
        // Check if it's a sub-command
        if let Some(signature) = scope.get_signature(&format!(
            "{} {}",
            lite_cmd.parts[0].item, lite_cmd.parts[1].item
        )) {
            let (mut internal_command, err) =
                parse_internal_command(&lite_cmd, scope, &signature, 1);

            error = error.or(err);
            internal_command.args.external_redirection = if end_of_pipeline {
                ExternalRedirection::None
            } else {
                ExternalRedirection::Stdout
            };
            return (Some(ClassifiedCommand::Internal(internal_command)), error);
        }
    }
    // Check if it's an internal command
    if let Some(signature) = scope.get_signature(&lite_cmd.parts[0].item) {
        if lite_cmd.parts[0].item == "def" {
            let error = parse_definition(&lite_cmd, scope);
            if error.is_some() {
                return (
                    Some(ClassifiedCommand::Expr(Box::new(garbage(lite_cmd.span())))),
                    error,
                );
            }
        }
        let (mut internal_command, err) = parse_internal_command(&lite_cmd, scope, &signature, 0);

        if internal_command.name == "source" {
            if lite_cmd.parts.len() != 2 {
                return (
                    Some(ClassifiedCommand::Internal(internal_command)),
                    Some(ParseError::argument_error(
                        lite_cmd.parts[0].clone(),
                        ArgumentError::MissingMandatoryPositional("a path for sourcing".into()),
                    )),
                );
            }
            if lite_cmd.parts[1].item.starts_with('$') {
                return (
                    Some(ClassifiedCommand::Internal(internal_command)),
                    Some(ParseError::mismatch(
                        "a filepath constant",
                        lite_cmd.parts[1].clone(),
                    )),
                );
            }
            if let Ok(contents) =
                std::fs::read_to_string(expand_path(&lite_cmd.parts[1].item).into_owned())
            {
                let _ = parse(&contents, 0, scope);
            } else {
                return (
                    Some(ClassifiedCommand::Internal(internal_command)),
                    Some(ParseError::argument_error(
                        lite_cmd.parts[1].clone(),
                        ArgumentError::BadValue("can't load source file".into()),
                    )),
                );
            }
        } else if lite_cmd.parts[0].item == "alias" {
            let error = parse_alias(&lite_cmd, scope);
            if error.is_none() {
                return (Some(ClassifiedCommand::Internal(internal_command)), None);
            } else {
                return (Some(ClassifiedCommand::Internal(internal_command)), error);
            }
        }

        error = error.or(err);
        internal_command.args.external_redirection = if end_of_pipeline {
            ExternalRedirection::None
        } else {
            ExternalRedirection::Stdout
        };
        (Some(ClassifiedCommand::Internal(internal_command)), error)
    } else {
        parse_external_call(&lite_cmd, end_of_pipeline, scope)
    }
}

/// Convert a lite-ly parsed pipeline into a fully classified pipeline, ready to be evaluated.
/// This conversion does error-recovery, so the result is allowed to be lossy. A lossy unit is designated as garbage.
/// Errors are returned as part of a side-car error rather than a Result to allow both error and lossy result simultaneously.
fn parse_pipeline(
    lite_pipeline: LitePipeline,
    scope: &dyn ParserScope,
) -> (Pipeline, Option<ParseError>) {
    let mut commands = Pipeline::new(lite_pipeline.span());
    let mut error = None;

    let mut iter = lite_pipeline.commands.into_iter().peekable();
    while let Some(lite_cmd) = iter.next() {
        let (call, err) = parse_call(lite_cmd, iter.peek().is_none(), scope);
        if error.is_none() {
            error = err;
        }
        if let Some(call) = call {
            commands.push(call);
        }
    }

    (commands, error)
}

type SpannedKeyValue = (Spanned<String>, Spanned<String>);

fn expand_shorthand_forms(
    lite_pipeline: &LitePipeline,
) -> (LitePipeline, Option<SpannedKeyValue>, Option<ParseError>) {
    if !lite_pipeline.commands.is_empty() {
        if lite_pipeline.commands[0].parts[0].contains('=')
            && !lite_pipeline.commands[0].parts[0].starts_with('$')
        {
            let assignment: Vec<_> = lite_pipeline.commands[0].parts[0].split('=').collect();
            if assignment.len() != 2 {
                (
                    lite_pipeline.clone(),
                    None,
                    Some(ParseError::mismatch(
                        "environment variable assignment",
                        lite_pipeline.commands[0].parts[0].clone(),
                    )),
                )
            } else {
                let original_span = lite_pipeline.commands[0].parts[0].span;
                let env_value = trim_quotes(assignment[1]);

                let (variable_name, value) = (assignment[0], env_value);
                let mut lite_pipeline = lite_pipeline.clone();

                if !lite_pipeline.commands[0].parts.len() > 1 {
                    let mut new_lite_command_parts = lite_pipeline.commands[0].parts.clone();
                    new_lite_command_parts.remove(0);

                    lite_pipeline.commands[0].parts = new_lite_command_parts;

                    (
                        lite_pipeline,
                        Some((
                            variable_name.to_string().spanned(original_span),
                            value.spanned(original_span),
                        )),
                        None,
                    )
                } else {
                    (
                        lite_pipeline.clone(),
                        None,
                        Some(ParseError::mismatch(
                            "a command following variable",
                            lite_pipeline.commands[0].parts[0].clone(),
                        )),
                    )
                }
            }
        } else {
            (lite_pipeline.clone(), None, None)
        }
    } else {
        (lite_pipeline.clone(), None, None)
    }
}

fn parse_alias(call: &LiteCommand, scope: &dyn ParserScope) -> Option<ParseError> {
    if call.parts.len() == 2 && (call.parts[1].item == "--help" || (call.parts[1].item == "-h")) {
        return None;
    }

    if call.parts.len() < 4 {
        return Some(ParseError::mismatch("alias", call.parts[0].clone()));
    }

    if call.parts[0].item != "alias" {
        return Some(ParseError::mismatch("alias", call.parts[0].clone()));
    }

    if call.parts[2].item != "=" {
        return Some(ParseError::mismatch("=", call.parts[2].clone()));
    }

    let name = call.parts[1].item.clone();
    let args: Vec<_> = call.parts.iter().skip(3).cloned().collect();

    scope.add_alias(&name, args);

    None
}

pub fn classify_block(
    lite_block: &LiteBlock,
    scope: &dyn ParserScope,
) -> (Arc<Block>, Option<ParseError>) {
    let mut output = Block::basic();
    let mut error = None;

    // Check for custom commands first
    for group in lite_block.block.iter() {
        for pipeline in &group.pipelines {
            for call in &pipeline.commands {
                if let Some(first) = call.parts.first() {
                    if first.item == "def" {
                        if pipeline.commands.len() > 1 && error.is_none() {
                            error = Some(ParseError::mismatch("definition", first.clone()));
                        }
                        parse_definition_prototype(call, scope);
                    }
                }
            }
        }
    }

    // Then the rest of the code
    for group in &lite_block.block {
        let mut out_group = Group::basic();
        for pipeline in &group.pipelines {
            let (pipeline, vars, err) = expand_shorthand_forms(pipeline);
            if error.is_none() {
                error = err;
            }

            let (out_pipe, err) = parse_pipeline(pipeline.clone(), scope);
            if error.is_none() {
                error = err;
            }

            let pipeline = if let Some(vars) = vars {
                let span = pipeline.span();
                let block = hir::Block::new(
                    Signature::new("<block>"),
                    vec![Group::new(vec![out_pipe.clone()], span)],
                    IndexMap::new(),
                    span,
                );
                let mut call = hir::Call::new(
                    Box::new(SpannedExpression {
                        expr: Expression::string("with-env".to_string()),
                        span,
                    }),
                    span,
                );
                call.positional = Some(vec![
                    SpannedExpression {
                        expr: Expression::List(vec![
                            SpannedExpression {
                                expr: Expression::string(vars.0.item),
                                span: vars.0.span,
                            },
                            SpannedExpression {
                                expr: Expression::string(vars.1.item),
                                span: vars.1.span,
                            },
                        ]),
                        span: Span::new(vars.0.span.start(), vars.1.span.end()),
                    },
                    SpannedExpression {
                        expr: Expression::Block(Arc::new(block)),
                        span,
                    },
                ]);
                let classified_with_env = ClassifiedCommand::Internal(InternalCommand {
                    name: "with-env".to_string(),
                    name_span: Span::unknown(),
                    args: call,
                });

                Pipeline {
                    list: vec![classified_with_env],
                    span,
                }
            } else {
                out_pipe
            };

            if !pipeline.list.is_empty() {
                out_group.push(pipeline);
            }
        }
        if !out_group.pipelines.is_empty() {
            output.push(out_group);
        }
    }

    let definitions = scope.get_definitions();
    for definition in definitions.into_iter() {
        let name = definition.params.name.clone();
        if !output.definitions.contains_key(&name) {
            output.definitions.insert(name, definition.clone());
        }
    }
    output.infer_params();

    (Arc::new(output), error)
}

pub fn parse(
    input: &str,
    span_offset: usize,
    scope: &dyn ParserScope,
) -> (Arc<Block>, Option<ParseError>) {
    let mut error = None;
    let (output, err) = lex(input, span_offset);
    if error.is_none() {
        error = err;
    }
    let (lite_block, err) = parse_block(output);
    if error.is_none() {
        error = err;
    }

    let (block, err) = classify_block(&lite_block, scope);
    if error.is_none() {
        error = err;
    }

    (block, error)
}

#[test]
fn unit_parse_byte_units() {
    struct TestCase {
        string: String,
        value: i64,
        unit: Unit,
    }

    let cases = [
        TestCase {
            string: String::from("108b"),
            value: 108,
            unit: Unit::Byte,
        },
        TestCase {
            string: String::from("0B"),
            value: 0,
            unit: Unit::Byte,
        },
        TestCase {
            string: String::from("10kb"),
            value: 10,
            unit: Unit::Kilobyte,
        },
        TestCase {
            string: String::from("16KB"),
            value: 16,
            unit: Unit::Kilobyte,
        },
        TestCase {
            string: String::from("99kB"),
            value: 99,
            unit: Unit::Kilobyte,
        },
        TestCase {
            string: String::from("27Kb"),
            value: 27,
            unit: Unit::Kilobyte,
        },
        TestCase {
            string: String::from("11Mb"),
            value: 11,
            unit: Unit::Megabyte,
        },
        TestCase {
            string: String::from("27mB"),
            value: 27,
            unit: Unit::Megabyte,
        },
        TestCase {
            string: String::from("811Gb"),
            value: 811,
            unit: Unit::Gigabyte,
        },
        TestCase {
            string: String::from("27gB"),
            value: 27,
            unit: Unit::Gigabyte,
        },
        TestCase {
            string: String::from("11Tb"),
            value: 11,
            unit: Unit::Terabyte,
        },
        TestCase {
            string: String::from("1027tB"),
            value: 1027,
            unit: Unit::Terabyte,
        },
        TestCase {
            string: String::from("11Pb"),
            value: 11,
            unit: Unit::Petabyte,
        },
        TestCase {
            string: String::from("27pB"),
            value: 27,
            unit: Unit::Petabyte,
        },
        TestCase {
            string: String::from("10kib"),
            value: 10,
            unit: Unit::Kibibyte,
        },
        TestCase {
            string: String::from("123KiB"),
            value: 123,
            unit: Unit::Kibibyte,
        },
        TestCase {
            string: String::from("24kiB"),
            value: 24,
            unit: Unit::Kibibyte,
        },
        TestCase {
            string: String::from("10mib"),
            value: 10,
            unit: Unit::Mebibyte,
        },
        TestCase {
            string: String::from("123MiB"),
            value: 123,
            unit: Unit::Mebibyte,
        },
        TestCase {
            string: String::from("10gib"),
            value: 10,
            unit: Unit::Gibibyte,
        },
        TestCase {
            string: String::from("123GiB"),
            value: 123,
            unit: Unit::Gibibyte,
        },
        TestCase {
            string: String::from("10tib"),
            value: 10,
            unit: Unit::Tebibyte,
        },
        TestCase {
            string: String::from("123TiB"),
            value: 123,
            unit: Unit::Tebibyte,
        },
        TestCase {
            string: String::from("10pib"),
            value: 10,
            unit: Unit::Pebibyte,
        },
        TestCase {
            string: String::from("123PiB"),
            value: 123,
            unit: Unit::Pebibyte,
        },
    ];

    for case in cases.iter() {
        let input_len = case.string.len();
        let value_len = case.value.to_string().len();
        let input = case.string.clone().spanned(Span::new(0, input_len));
        let result = parse_filesize(&input);
        assert_eq!(result.1, None);
        assert_eq!(
            result.0.expr,
            Expression::unit(
                Spanned {
                    span: Span::new(0, value_len),
                    item: case.value
                },
                Spanned {
                    span: Span::new(value_len, input_len),
                    item: case.unit
                }
            )
        );
    }
}

#[test]
fn unit_parse_byte_units_decimal() {
    struct TestCase {
        string: String,
        value: i64,
        value_str: String,
        unit: Unit,
    }

    let cases = [
        TestCase {
            string: String::from("0.25KB"),
            value: 250,
            value_str: String::from("0.25"),
            unit: Unit::Byte,
        },
        TestCase {
            string: String::from("2.5Mb"),
            value: 2500,
            value_str: String::from("2.5"),
            unit: Unit::Kilobyte,
        },
        TestCase {
            string: String::from("0.5Gb"),
            value: 500,
            value_str: String::from("0.5"),
            unit: Unit::Megabyte,
        },
        TestCase {
            string: String::from("811.5Gb"),
            value: 811500,
            value_str: String::from("811.5"),
            unit: Unit::Megabyte,
        },
        TestCase {
            string: String::from("11.5Tb"),
            value: 11500,
            value_str: String::from("11.5"),
            unit: Unit::Gigabyte,
        },
        TestCase {
            string: String::from("12.5Pb"),
            value: 12500,
            value_str: String::from("12.5"),
            unit: Unit::Terabyte,
        },
        TestCase {
            string: String::from("10.5kib"),
            value: 10752,
            value_str: String::from("10.5"),
            unit: Unit::Byte,
        },
        TestCase {
            string: String::from("0.5mib"),
            value: 512,
            value_str: String::from("0.5"),
            unit: Unit::Kibibyte,
        },
        TestCase {
            string: String::from("3.25gib"),
            value: 3328,
            value_str: String::from("3.25"),
            unit: Unit::Mebibyte,
        },
    ];

    for case in cases.iter() {
        let input_len = case.string.len();
        let value_len = case.value_str.to_string().len();
        let input = case.string.clone().spanned(Span::new(0, input_len));
        let result = parse_filesize(&input);
        assert_eq!(result.1, None);
        assert_eq!(
            result.0.expr,
            Expression::unit(
                Spanned {
                    span: Span::new(0, value_len),
                    item: case.value
                },
                Spanned {
                    span: Span::new(value_len, input_len),
                    item: case.unit
                }
            )
        );
    }
}
