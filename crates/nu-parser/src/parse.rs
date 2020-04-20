use std::path::Path;

use crate::lite_parse::{lite_parse, LiteCommand, LitePipeline};
use crate::path::expand_path;
use crate::signature::SignatureRegistry;
use log::trace;
use nu_errors::{ArgumentError, ParseError};
use nu_protocol::hir::{
    self, Binary, ClassifiedCommand, ClassifiedPipeline, Commands, Expression, Flag, FlagKind,
    InternalCommand, Member, NamedArguments, Operator, SpannedExpression, Unit,
};
use nu_protocol::{NamedType, PositionalType, Signature, SyntaxShape, UnspannedPathMember};
use nu_source::{Span, Spanned, SpannedItem};
use num_bigint::BigInt;

/// Parses a simple column path, one without a variable (implied or explicit) at the head
fn parse_simple_column_path(lite_arg: &Spanned<String>) -> (SpannedExpression, Option<ParseError>) {
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

            if let Ok(row_number) = current_part.parse::<u64>() {
                output.push(Member::Int(BigInt::from(row_number), part_span));
            } else {
                let trimmed = trim_quotes(&current_part);
                output.push(Member::Bare(trimmed.clone().spanned(part_span)));
            }
            current_part.clear();
            // Note: I believe this is safe because of the delimiter we're using, but if we get fancy with
            // unicode we'll need to change this
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
        if let Ok(row_number) = current_part.parse::<u64>() {
            output.push(Member::Int(BigInt::from(row_number), part_span));
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
fn parse_full_column_path(lite_arg: &Spanned<String>) -> (SpannedExpression, Option<ParseError>) {
    let mut delimiter = '.';
    let mut inside_delimiter = false;
    let mut output = vec![];
    let mut current_part = String::new();
    let mut start_index = 0;
    let mut last_index = 0;

    let mut head = None;

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

            if head.is_none() && current_part.clone().starts_with('$') {
                // We have the variable head
                head = Some(Expression::variable(current_part.clone(), part_span))
            } else if let Ok(row_number) = current_part.parse::<u64>() {
                output.push(
                    UnspannedPathMember::Int(BigInt::from(row_number)).into_path_member(part_span),
                );
            } else {
                let current_part = trim_quotes(&current_part);
                output.push(
                    UnspannedPathMember::String(current_part.clone()).into_path_member(part_span),
                );
            }
            current_part.clear();
            // Note: I believe this is safe because of the delimiter we're using, but if we get fancy with
            // unicode we'll need to change this
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
            if current_part.starts_with('$') {
                // We have the variable head
                head = Some(Expression::variable(current_part, lite_arg.span));
            } else if let Ok(row_number) = current_part.parse::<u64>() {
                output.push(
                    UnspannedPathMember::Int(BigInt::from(row_number)).into_path_member(part_span),
                );
            } else {
                let current_part = trim_quotes(&current_part);
                output.push(UnspannedPathMember::String(current_part).into_path_member(part_span));
            }
        } else if let Ok(row_number) = current_part.parse::<u64>() {
            output.push(
                UnspannedPathMember::Int(BigInt::from(row_number)).into_path_member(part_span),
            );
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
            None,
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
            None,
        )
    }
}

fn trim_quotes(input: &str) -> String {
    let mut chars = input.chars();

    match (chars.next(), chars.next_back()) {
        (Some('\''), Some('\'')) => chars.collect(),
        (Some('"'), Some('"')) => chars.collect(),
        _ => input.to_string(),
    }
}

/// Parse a numeric range
fn parse_range(lite_arg: &Spanned<String>) -> (SpannedExpression, Option<ParseError>) {
    let numbers: Vec<_> = lite_arg.item.split("..").collect();

    if numbers.len() != 2 {
        (
            garbage(lite_arg.span),
            Some(ParseError::mismatch("range", lite_arg.clone())),
        )
    } else if let Ok(lhs) = numbers[0].parse::<i64>() {
        if let Ok(rhs) = numbers[1].parse::<i64>() {
            (
                SpannedExpression::new(
                    Expression::range(
                        SpannedExpression::new(Expression::integer(lhs), lite_arg.span),
                        lite_arg.span,
                        SpannedExpression::new(Expression::integer(rhs), lite_arg.span),
                    ),
                    lite_arg.span,
                ),
                None,
            )
        } else {
            (
                garbage(lite_arg.span),
                Some(ParseError::mismatch("range", lite_arg.clone())),
            )
        }
    } else {
        (
            garbage(lite_arg.span),
            Some(ParseError::mismatch("range", lite_arg.clone())),
        )
    }
}

/// Parse any allowed operator, including word-based operators
fn parse_operator(lite_arg: &Spanned<String>) -> (SpannedExpression, Option<ParseError>) {
    let operator = if lite_arg.item == "==" {
        Operator::Equal
    } else if lite_arg.item == "!=" {
        Operator::NotEqual
    } else if lite_arg.item == "<" {
        Operator::LessThan
    } else if lite_arg.item == "<=" {
        Operator::LessThanOrEqual
    } else if lite_arg.item == ">" {
        Operator::GreaterThan
    } else if lite_arg.item == ">=" {
        Operator::GreaterThanOrEqual
    } else if lite_arg.item == "=~" {
        Operator::Contains
    } else if lite_arg.item == "!~" {
        Operator::NotContains
    } else if lite_arg.item == "+" {
        Operator::Plus
    } else if lite_arg.item == "-" {
        Operator::Minus
    } else if lite_arg.item == "*" {
        Operator::Multiply
    } else if lite_arg.item == "/" {
        Operator::Divide
    } else if lite_arg.item == "in:" {
        Operator::In
    } else if lite_arg.item == "&&" {
        Operator::And
    } else if lite_arg.item == "||" {
        Operator::Or
    } else {
        return (
            garbage(lite_arg.span),
            Some(ParseError::mismatch("operator", lite_arg.clone())),
        );
    };

    (
        SpannedExpression::new(Expression::operator(operator), lite_arg.span),
        None,
    )
}

/// Parse a unit type, eg '10kb'
fn parse_unit(lite_arg: &Spanned<String>) -> (SpannedExpression, Option<ParseError>) {
    let unit_groups = [
        (Unit::Byte, vec!["b", "B"]),
        (Unit::Kilobyte, vec!["kb", "KB", "Kb"]),
        (Unit::Megabyte, vec!["mb", "MB", "Mb"]),
        (Unit::Gigabyte, vec!["gb", "GB", "Gb"]),
        (Unit::Terabyte, vec!["tb", "TB", "Tb"]),
        (Unit::Petabyte, vec!["pb", "PB", "Pb"]),
        (Unit::Second, vec!["s"]),
        (Unit::Minute, vec!["m"]),
        (Unit::Hour, vec!["h"]),
        (Unit::Day, vec!["d"]),
        (Unit::Week, vec!["w"]),
        (Unit::Month, vec!["M"]),
        (Unit::Year, vec!["y"]),
    ];

    for unit_group in unit_groups.iter() {
        for unit in unit_group.1.iter() {
            if lite_arg.item.ends_with(unit) {
                let mut lhs = lite_arg.item.clone();

                for _ in 0..unit.len() {
                    lhs.pop();
                }

                // these units are allowed to signed
                if let Ok(x) = lhs.parse::<i64>() {
                    let lhs_span =
                        Span::new(lite_arg.span.start(), lite_arg.span.start() + lhs.len());
                    let unit_span =
                        Span::new(lite_arg.span.start() + lhs.len(), lite_arg.span.end());
                    return (
                        SpannedExpression::new(
                            Expression::unit(x.spanned(lhs_span), unit_group.0.spanned(unit_span)),
                            lite_arg.span,
                        ),
                        None,
                    );
                }
            }
        }
    }

    (
        garbage(lite_arg.span),
        Some(ParseError::mismatch("unit", lite_arg.clone())),
    )
}

/// Parses the given argument using the shape as a guide for how to correctly parse the argument
fn parse_arg(
    expected_type: SyntaxShape,
    registry: &dyn SignatureRegistry,
    lite_arg: &Spanned<String>,
) -> (SpannedExpression, Option<ParseError>) {
    if lite_arg.item.starts_with('$') {
        return parse_full_column_path(&lite_arg);
    }

    match expected_type {
        SyntaxShape::Number => {
            if let Ok(x) = lite_arg.item.parse::<i64>() {
                (
                    SpannedExpression::new(Expression::integer(x), lite_arg.span),
                    None,
                )
            } else if let Ok(x) = lite_arg.item.parse::<f64>() {
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
            } else {
                (
                    garbage(lite_arg.span),
                    Some(ParseError::mismatch("number", lite_arg.clone())),
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
        SyntaxShape::Pattern => {
            let trimmed = trim_quotes(&lite_arg.item);
            let expanded = expand_path(&trimmed);
            (
                SpannedExpression::new(Expression::pattern(expanded), lite_arg.span),
                None,
            )
        }

        SyntaxShape::Range => parse_range(&lite_arg),
        SyntaxShape::Operator => parse_operator(&lite_arg),
        SyntaxShape::Unit => parse_unit(&lite_arg),
        SyntaxShape::Path => {
            let trimmed = trim_quotes(&lite_arg.item);
            let expanded = expand_path(&trimmed);
            let path = Path::new(&expanded);
            (
                SpannedExpression::new(Expression::FilePath(path.to_path_buf()), lite_arg.span),
                None,
            )
        }
        SyntaxShape::ColumnPath => parse_simple_column_path(lite_arg),
        SyntaxShape::FullColumnPath => parse_full_column_path(lite_arg),
        SyntaxShape::Any => {
            let shapes = vec![
                SyntaxShape::Int,
                SyntaxShape::Number,
                SyntaxShape::Range,
                SyntaxShape::Unit,
                SyntaxShape::Block,
                SyntaxShape::Table,
                SyntaxShape::String,
            ];
            for shape in shapes.iter() {
                if let (s, None) = parse_arg(*shape, registry, lite_arg) {
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
                    let mut error = None;

                    // We haven't done much with the inner string, so let's go ahead and work with it
                    let lite_pipeline = match lite_parse(&string, lite_arg.span.start() + 1) {
                        Ok(lp) => lp,
                        Err(e) => return (garbage(lite_arg.span), Some(e)),
                    };

                    let mut output = vec![];
                    for lite_inner in &lite_pipeline.commands {
                        let (arg, err) = parse_arg(SyntaxShape::Any, registry, &lite_inner.name);

                        output.push(arg);
                        if error.is_none() {
                            error = err;
                        }

                        for arg in &lite_inner.args {
                            let (arg, err) = parse_arg(SyntaxShape::Any, registry, &arg);
                            output.push(arg);

                            if error.is_none() {
                                error = err;
                            }
                        }
                    }

                    (
                        SpannedExpression::new(Expression::List(output), lite_arg.span),
                        error,
                    )
                }
                _ => (
                    garbage(lite_arg.span),
                    Some(ParseError::mismatch("table", lite_arg.clone())),
                ),
            }
        }
        SyntaxShape::Block | SyntaxShape::Math => {
            // Blocks have one of two forms: the literal block and the implied block
            // To parse a literal block, we need to detect that what we have is itself a block
            let mut chars = lite_arg.item.chars();

            match (chars.next(), chars.next_back()) {
                (Some('{'), Some('}')) => {
                    // We have a literal block
                    let string: String = chars.collect();

                    // We haven't done much with the inner string, so let's go ahead and work with it
                    let lite_pipeline = match lite_parse(&string, lite_arg.span.start() + 1) {
                        Ok(lp) => lp,
                        Err(e) => return (garbage(lite_arg.span), Some(e)),
                    };

                    let classified_block = classify_pipeline(&lite_pipeline, registry);
                    let error = classified_block.failed;

                    (
                        SpannedExpression::new(
                            Expression::Block(classified_block.commands),
                            lite_arg.span,
                        ),
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

/// Match the available flags in a signature with what the user provided. This will check both long-form flags (--full) and shorthand flags (-f)
/// This also allows users to provide a group of shorthand flags (-af) that correspond to multiple shorthand flags at once.
fn get_flags_from_flag(
    signature: &nu_protocol::Signature,
    cmd: &Spanned<String>,
    arg: &Spanned<String>,
) -> (Vec<(String, NamedType)>, Option<ParseError>) {
    if arg.item.starts_with('-') {
        // It's a flag (or set of flags)
        let mut output = vec![];
        let mut error = None;

        let remainder: String = arg.item.chars().skip(1).collect();

        if remainder.starts_with('-') {
            // Long flag expected
            let remainder: String = remainder.chars().skip(1).collect();
            if let Some((named_type, _)) = signature.named.get(&remainder) {
                output.push((remainder.clone(), named_type.clone()));
            } else {
                error = Some(ParseError::argument_error(
                    cmd.clone(),
                    ArgumentError::UnexpectedFlag(arg.clone()),
                ));
            }
        } else {
            // Short flag(s) expected
            let mut starting_pos = arg.span.start() + 1;
            for c in remainder.chars() {
                let mut found = false;
                for (full_name, named_arg) in signature.named.iter() {
                    if Some(c) == named_arg.0.get_short() {
                        found = true;
                        output.push((full_name.clone(), named_arg.0.clone()));
                        break;
                    }
                }

                if !found {
                    error = Some(ParseError::argument_error(
                        cmd.clone(),
                        ArgumentError::UnexpectedFlag(
                            arg.item
                                .clone()
                                .spanned(Span::new(starting_pos, starting_pos + c.len_utf8())),
                        ),
                    ));
                }

                starting_pos += c.len_utf8();
            }
        }

        (output, error)
    } else {
        // It's not a flag, so don't bother with it
        (vec![], None)
    }
}

/// This is a bit of a "fix-up" of previously parsed areas. In cases where we're in shorthand mode (eg in the `where` command), we need
/// to use the original source to parse a column path. Without it, we'll lose a little too much information to parse it correctly. As we'll
/// only know we were on the left-hand side of an expression after we do the full math parse, we need to do this step after rather than during
/// the initial parse.
fn shorthand_reparse(
    left: SpannedExpression,
    orig_left: Option<Spanned<String>>,
    registry: &dyn SignatureRegistry,
    shorthand_mode: bool,
) -> (SpannedExpression, Option<ParseError>) {
    // If we're in shorthand mode, we need to reparse the left-hand side if possible
    if shorthand_mode {
        if let Some(orig_left) = orig_left {
            parse_arg(SyntaxShape::FullColumnPath, registry, &orig_left)
        } else {
            (left, None)
        }
    } else {
        (left, None)
    }
}

fn parse_parenthesized_expression(
    lite_arg: &Spanned<String>,
    registry: &dyn SignatureRegistry,
    shorthand_mode: bool,
) -> (SpannedExpression, Option<ParseError>) {
    let mut chars = lite_arg.item.chars();

    match (chars.next(), chars.next_back()) {
        (Some('('), Some(')')) => {
            // We have a literal row
            let string: String = chars.collect();

            // We haven't done much with the inner string, so let's go ahead and work with it
            let mut lite_pipeline = match lite_parse(&string, lite_arg.span.start() + 1) {
                Ok(lp) => lp,
                Err(e) => return (garbage(lite_arg.span), Some(e)),
            };

            let mut collection = vec![];
            for lite_cmd in lite_pipeline.commands.iter_mut() {
                collection.push(lite_cmd.name.clone());
                collection.append(&mut lite_cmd.args);
            }
            let (_, expr, err) =
                parse_math_expression(0, &collection[..], registry, shorthand_mode);
            (expr, err)
        }
        _ => (
            garbage(lite_arg.span),
            Some(ParseError::mismatch("table", lite_arg.clone())),
        ),
    }
}

fn parse_possibly_parenthesized(
    lite_arg: &Spanned<String>,
    registry: &dyn SignatureRegistry,
    shorthand_mode: bool,
) -> (
    (Option<Spanned<String>>, SpannedExpression),
    Option<ParseError>,
) {
    if lite_arg.item.starts_with('(') {
        let (lhs, err) = parse_parenthesized_expression(lite_arg, registry, shorthand_mode);
        ((None, lhs), err)
    } else {
        let (lhs, err) = parse_arg(SyntaxShape::Any, registry, lite_arg);
        ((Some(lite_arg.clone()), lhs), err)
    }
}

/// Handle parsing math expressions, complete with working with the precedence of the operators
fn parse_math_expression(
    incoming_idx: usize,
    lite_args: &[Spanned<String>],
    registry: &dyn SignatureRegistry,
    shorthand_mode: bool,
) -> (usize, SpannedExpression, Option<ParseError>) {
    // Precedence parsing is included
    // Some notes:
    //   * short_hand mode means that the left-hand side of an expression can point to a column-path. To make this possible,
    //     we parse as normal, but then go back and when we detect a left-hand side, reparse that value if it's a string
    //   * parens are handled earlier, so they're not handled explicitly here

    let mut idx = 0;
    let mut error = None;

    let mut working_exprs = vec![];
    let mut prec = vec![];

    let (lhs_working_expr, err) =
        parse_possibly_parenthesized(&lite_args[idx], registry, shorthand_mode);

    if error.is_none() {
        error = err;
    }
    working_exprs.push(lhs_working_expr);

    idx += 1;

    prec.push(0);

    while idx < lite_args.len() {
        let (op, err) = parse_arg(SyntaxShape::Operator, registry, &lite_args[idx]);
        if error.is_none() {
            error = err;
        }
        idx += 1;

        if idx < lite_args.len() {
            trace!(
                "idx: {} working_exprs: {:#?} prec: {:?}",
                idx,
                working_exprs,
                prec
            );

            let (rhs_working_expr, err) =
                parse_possibly_parenthesized(&lite_args[idx], registry, shorthand_mode);

            if error.is_none() {
                error = err;
            }

            let next_prec = op.precedence();

            if !prec.is_empty() && next_prec > *prec.last().expect("this shouldn't happen") {
                prec.push(next_prec);
                working_exprs.push((None, op));
                working_exprs.push(rhs_working_expr);
            } else {
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
                    let (orig_left, left) =
                        working_exprs.pop().expect("This shouldn't be possible");

                    // If we're in shorthand mode, we need to reparse the left-hand side if possibe
                    let (left, err) = shorthand_reparse(left, orig_left, registry, shorthand_mode);
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
            }

            idx += 1;
        } else {
            if error.is_none() {
                error = Some(ParseError::argument_error(
                    lite_args[idx - 1].clone(),
                    ArgumentError::MissingMandatoryPositional("right hand side".into()),
                ));
            }
            working_exprs.push((None, garbage(op.span)));
            working_exprs.push((None, garbage(op.span)));
            prec.push(0);
        }
    }

    while working_exprs.len() >= 3 {
        // Pop 3 and create and expression, push and repeat
        let (_, right) = working_exprs.pop().expect("This shouldn't be possible");
        let (_, op) = working_exprs.pop().expect("This shouldn't be possible");
        let (orig_left, left) = working_exprs.pop().expect("This shouldn't be possible");

        let (left, err) = shorthand_reparse(left, orig_left, registry, shorthand_mode);
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
    let (left, err) = shorthand_reparse(left, orig_left, registry, shorthand_mode);
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
    registry: &dyn SignatureRegistry,
) -> (usize, SpannedExpression, Option<ParseError>) {
    let mut idx = idx;
    let mut error = None;
    let arg = match positional_type {
        PositionalType::Mandatory(_, SyntaxShape::Math)
        | PositionalType::Optional(_, SyntaxShape::Math) => {
            // A condition can take up multiple arguments, as we build the operation as <arg> <operator> <arg>
            // We need to do this here because in parse_arg, we have access to only one arg at a time

            if idx < lite_cmd.args.len() {
                if lite_cmd.args[idx].item.starts_with('{') {
                    // It's an explicit math expression, so parse it deeper in
                    let (arg, err) = parse_arg(SyntaxShape::Math, registry, &lite_cmd.args[idx]);
                    if error.is_none() {
                        error = err;
                    }
                    arg
                } else {
                    let (new_idx, arg, err) =
                        parse_math_expression(idx, &lite_cmd.args[idx..], registry, true);

                    let span = arg.span;
                    let mut commands = hir::Commands::new(span);
                    commands.push(ClassifiedCommand::Expr(Box::new(arg)));

                    let arg = SpannedExpression::new(Expression::Block(commands), span);

                    idx = new_idx;
                    if error.is_none() {
                        error = err;
                    }
                    arg
                }
            } else {
                if error.is_none() {
                    error = Some(ParseError::argument_error(
                        lite_cmd.name.clone(),
                        ArgumentError::MissingMandatoryPositional("condition".into()),
                    ))
                }
                garbage(lite_cmd.span())
            }
        }
        PositionalType::Mandatory(_, shape) | PositionalType::Optional(_, shape) => {
            let (arg, err) = parse_arg(*shape, registry, &lite_cmd.args[idx]);
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
    registry: &dyn SignatureRegistry,
    signature: &Signature,
) -> (InternalCommand, Option<ParseError>) {
    // This is a known internal command, so we need to work with the arguments and parse them according to the expected types
    let mut internal_command = InternalCommand::new(
        lite_cmd.name.item.clone(),
        lite_cmd.name.span,
        lite_cmd.span(),
    );
    internal_command.args.set_initial_flags(&signature);

    let mut idx = 0;
    let mut current_positional = 0;
    let mut named = NamedArguments::new();
    let mut positional = vec![];
    let mut error = None;

    while idx < lite_cmd.args.len() {
        if lite_cmd.args[idx].item.starts_with('-') && lite_cmd.args[idx].item.len() > 1 {
            let (named_types, err) =
                get_flags_from_flag(&signature, &lite_cmd.name, &lite_cmd.args[idx]);

            if err.is_none() {
                for (full_name, named_type) in &named_types {
                    match named_type {
                        NamedType::Mandatory(_, shape) | NamedType::Optional(_, shape) => {
                            if idx == lite_cmd.args.len() {
                                // Oops, we're missing the argument to our named argument
                                if error.is_none() {
                                    error = Some(ParseError::argument_error(
                                        lite_cmd.name.clone(),
                                        ArgumentError::MissingValueForName(format!("{:?}", shape)),
                                    ));
                                }
                            } else {
                                idx += 1;
                                if lite_cmd.args.len() > idx {
                                    let (arg, err) =
                                        parse_arg(*shape, registry, &lite_cmd.args[idx]);
                                    named.insert_mandatory(
                                        full_name.clone(),
                                        lite_cmd.args[idx - 1].span,
                                        arg,
                                    );

                                    if error.is_none() {
                                        error = err;
                                    }
                                } else if error.is_none() {
                                    error = Some(ParseError::argument_error(
                                        lite_cmd.name.clone(),
                                        ArgumentError::MissingValueForName(full_name.to_owned()),
                                    ));
                                }
                            }
                        }
                        NamedType::Switch(_) => {
                            named.insert_switch(
                                full_name.clone(),
                                Some(Flag::new(FlagKind::Longhand, lite_cmd.args[idx].span)),
                            );
                        }
                    }
                }
            } else {
                positional.push(garbage(lite_cmd.args[idx].span));

                if error.is_none() {
                    error = err;
                }
            }
        } else if signature.positional.len() > current_positional {
            let arg = {
                let (new_idx, expr, err) = parse_positional_argument(
                    idx,
                    &lite_cmd,
                    &signature.positional[current_positional].0,
                    registry,
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
            let (arg, err) = parse_arg(*rest_type, registry, &lite_cmd.args[idx]);
            if error.is_none() {
                error = err;
            }

            positional.push(arg);
            current_positional += 1;
        } else {
            positional.push(garbage(lite_cmd.args[idx].span));

            if error.is_none() {
                error = Some(ParseError::argument_error(
                    lite_cmd.name.clone(),
                    ArgumentError::UnexpectedArgument(lite_cmd.args[idx].clone()),
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
        let (_, name) = &signature.positional[positional.len()];
        error = Some(ParseError::argument_error(
            lite_cmd.name.clone(),
            ArgumentError::MissingMandatoryPositional(name.to_owned()),
        ));
    }

    if !named.is_empty() {
        internal_command.args.named = Some(named);
    }

    if !positional.is_empty() {
        internal_command.args.positional = Some(positional);
    }

    (internal_command, error)
}

/// Convert a lite-ly parsed pipeline into a fully classified pipeline, ready to be evaluated.
/// This conversion does error-recovery, so the result is allowed to be lossy. A lossy unit is designated as garbage.
/// Errors are returned as part of a side-car error rather than a Result to allow both error and lossy result simultaneously.
pub fn classify_pipeline(
    lite_pipeline: &LitePipeline,
    registry: &dyn SignatureRegistry,
) -> ClassifiedPipeline {
    // FIXME: fake span
    let mut commands = Commands::new(Span::new(0, 0));
    let mut error = None;

    let mut iter = lite_pipeline.commands.iter().peekable();
    while let Some(lite_cmd) = iter.next() {
        if lite_cmd.name.item.starts_with('^') {
            let name = lite_cmd
                .name
                .clone()
                .map(|v| v.chars().skip(1).collect::<String>());

            // TODO this is the same as the `else` branch below, only the name differs. Find a way
            //      to share this functionality.
            let name_iter = std::iter::once(name);
            let args = name_iter.chain(lite_cmd.args.clone().into_iter());
            let args = arguments_from_string_iter(args);

            commands.push(ClassifiedCommand::Internal(InternalCommand {
                name: "run_external".to_string(),
                name_span: Span::unknown(),
                args: hir::Call {
                    head: Box::new(SpannedExpression {
                        expr: Expression::string("run_external".to_string()),
                        span: Span::unknown(),
                    }),
                    positional: Some(args),
                    named: None,
                    span: Span::unknown(),
                    is_last: iter.peek().is_none(),
                },
            }))
        } else if lite_cmd.name.item == "=" {
            let expr = if !lite_cmd.args.is_empty() {
                let (_, expr, err) = parse_math_expression(0, &lite_cmd.args[0..], registry, false);
                error = error.or(err);
                expr
            } else {
                error = error.or_else(|| {
                    Some(ParseError::argument_error(
                        lite_cmd.name.clone(),
                        ArgumentError::MissingMandatoryPositional("an expression".into()),
                    ))
                });
                garbage(lite_cmd.span())
            };
            commands.push(ClassifiedCommand::Expr(Box::new(expr)))
        } else if let Some(signature) = registry.get(&lite_cmd.name.item) {
            let (internal_command, err) = parse_internal_command(&lite_cmd, registry, &signature);

            error = error.or(err);
            commands.push(ClassifiedCommand::Internal(internal_command))
        } else {
            let name = lite_cmd.name.clone().map(|v| {
                let trimmed = trim_quotes(&v);
                expand_path(&trimmed)
            });

            let name_iter = std::iter::once(name);
            let args = name_iter.chain(lite_cmd.args.clone().into_iter());
            let args = arguments_from_string_iter(args);

            commands.push(ClassifiedCommand::Internal(InternalCommand {
                name: "run_external".to_string(),
                name_span: Span::unknown(),
                args: hir::Call {
                    head: Box::new(SpannedExpression {
                        expr: Expression::string("run_external".to_string()),
                        span: Span::unknown(),
                    }),
                    positional: Some(args),
                    named: None,
                    span: Span::unknown(),
                    is_last: iter.peek().is_none(),
                },
            }))
        }
    }

    ClassifiedPipeline::new(commands, error)
}

/// Parse out arguments from spanned expressions
pub fn arguments_from_string_iter(
    iter: impl Iterator<Item = Spanned<String>>,
) -> Vec<SpannedExpression> {
    iter.map(|v| {
        // TODO parse_full_column_path
        SpannedExpression {
            expr: Expression::string(v.to_string()),
            span: v.span,
        }
    })
    .collect::<Vec<_>>()
}

/// Easy shorthand function to create a garbage expression at the given span
pub fn garbage(span: Span) -> SpannedExpression {
    SpannedExpression::new(Expression::Garbage, span)
}
