use std::path::Path;

use nu_errors::{ArgumentError, ParseError};
//use crate::hir::*;
use crate::hir::{
    Binary, CompareOperator, Expression, Flag, FlagKind, Member, NamedArguments, SpannedExpression,
    Unit,
};
use crate::lite_parse::{lite_parse, LiteCommand, LitePipeline};
use crate::signature::SignatureRegistry;
use crate::{ExternalArg, ExternalArgs, ExternalCommand};
use nu_protocol::{NamedType, PositionalType, Signature, SyntaxShape, UnspannedPathMember};
use nu_source::{Span, Spanned, SpannedItem, Tag};
use num_bigint::BigInt;

#[derive(Debug, Clone)]
pub struct InternalCommand {
    pub name: String,
    pub name_span: Span,
    pub args: crate::hir::Call,
}

impl InternalCommand {
    pub fn new(name: String, name_span: Span, full_span: Span) -> InternalCommand {
        InternalCommand {
            name: name.clone(),
            name_span,
            args: crate::hir::Call::new(
                Box::new(SpannedExpression::new(Expression::string(name), name_span)),
                full_span,
            ),
        }
    }
}

#[derive(Debug, Clone)]
pub enum ClassifiedCommand {
    #[allow(unused)]
    Comparison(
        Box<SpannedExpression>,
        Box<SpannedExpression>,
        Box<SpannedExpression>,
    ),
    #[allow(unused)]
    Dynamic(crate::hir::Call),
    Internal(InternalCommand),
    External(crate::ExternalCommand),
    Error(ParseError),
}

#[derive(Debug, Clone)]
pub struct Commands {
    pub list: Vec<ClassifiedCommand>,
    pub span: Span,
}

impl Commands {
    pub fn new(span: Span) -> Commands {
        Commands { list: vec![], span }
    }

    pub fn push(&mut self, command: ClassifiedCommand) {
        self.list.push(command);
    }
}

#[derive(Debug, Clone)]
pub struct ClassifiedPipeline {
    pub commands: Commands,
    // this is not a Result to make it crystal clear that these shapes
    // aren't intended to be used directly with `?`
    pub failed: Option<ParseError>,
}

impl ClassifiedPipeline {
    pub fn new(commands: Commands, failed: Option<ParseError>) -> ClassifiedPipeline {
        ClassifiedPipeline { commands, failed }
    }
}

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

fn parse_operator(lite_arg: &Spanned<String>) -> (SpannedExpression, Option<ParseError>) {
    let operator = if lite_arg.item == "==" {
        CompareOperator::Equal
    } else if lite_arg.item == "!=" {
        CompareOperator::NotEqual
    } else if lite_arg.item == "<" {
        CompareOperator::LessThan
    } else if lite_arg.item == "<=" {
        CompareOperator::LessThanOrEqual
    } else if lite_arg.item == ">" {
        CompareOperator::GreaterThan
    } else if lite_arg.item == ">=" {
        CompareOperator::GreaterThanOrEqual
    } else if lite_arg.item == "=~" {
        CompareOperator::Contains
    } else if lite_arg.item == "!~" {
        CompareOperator::NotContains
    } else {
        return (
            garbage(lite_arg.span),
            Some(ParseError::mismatch(
                "comparison operator",
                lite_arg.clone(),
            )),
        );
    };

    (
        SpannedExpression::new(Expression::operator(operator), lite_arg.span),
        None,
    )
}

fn parse_unit(lite_arg: &Spanned<String>) -> (SpannedExpression, Option<ParseError>) {
    let unit_groups = [
        (Unit::Byte, true, vec!["b", "B"]),
        (Unit::Kilobyte, true, vec!["kb", "KB", "Kb"]),
        (Unit::Megabyte, true, vec!["mb", "MB", "Mb"]),
        (Unit::Gigabyte, true, vec!["gb", "GB", "Gb"]),
        (Unit::Terabyte, true, vec!["tb", "TB", "Tb"]),
        (Unit::Petabyte, true, vec!["pb", "PB", "Pb"]),
        (Unit::Second, false, vec!["s"]),
        (Unit::Minute, false, vec!["m"]),
        (Unit::Hour, false, vec!["h"]),
        (Unit::Day, false, vec!["d"]),
        (Unit::Week, false, vec!["w"]),
        (Unit::Month, false, vec!["M"]),
        (Unit::Year, false, vec!["y"]),
    ];

    for unit_group in unit_groups.iter() {
        for unit in unit_group.2.iter() {
            if lite_arg.item.ends_with(unit) {
                let mut lhs = lite_arg.item.clone();

                for _ in 0..unit.len() {
                    lhs.pop();
                }

                if unit_group.1 {
                    // these units are allowed to signed
                    if let Ok(x) = lhs.parse::<i64>() {
                        let lhs_span =
                            Span::new(lite_arg.span.start(), lite_arg.span.start() + lhs.len());
                        let unit_span =
                            Span::new(lite_arg.span.start() + lhs.len(), lite_arg.span.end());
                        return (
                            SpannedExpression::new(
                                Expression::unit(
                                    x.spanned(lhs_span),
                                    unit_group.0.spanned(unit_span),
                                ),
                                lite_arg.span,
                            ),
                            None,
                        );
                    }
                } else {
                    // these units are unsigned
                    if let Ok(x) = lhs.parse::<u64>() {
                        let lhs_span =
                            Span::new(lite_arg.span.start(), lite_arg.span.start() + lhs.len());
                        let unit_span =
                            Span::new(lite_arg.span.start() + lhs.len(), lite_arg.span.end());
                        return (
                            SpannedExpression::new(
                                Expression::unit(
                                    (x as i64).spanned(lhs_span),
                                    unit_group.0.spanned(unit_span),
                                ),
                                lite_arg.span,
                            ),
                            None,
                        );
                    }
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
            let expanded = shellexpand::tilde(&trimmed).to_string();
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
            let expanded = shellexpand::tilde(&trimmed).to_string();
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
        SyntaxShape::Block => {
            // Blocks have one of two forms: the literal block and the implied block
            // To parse a literal block, we need to detect that what we have is itself a block
            let mut chars = lite_arg.item.chars();

            match (chars.next(), chars.next_back()) {
                (Some('{'), Some('}')) => {
                    // We have a literal block
                    let string: String = chars.collect();
                    let mut error = None;

                    // We haven't done much with the inner string, so let's go ahead and work with it
                    let lite_pipeline = match lite_parse(&string, lite_arg.span.start() + 1) {
                        Ok(lp) => lp,
                        Err(e) => return (garbage(lite_arg.span), Some(e)),
                    };
                    //let pipeline = parse(&lite_pipeline, definitions)?;

                    // For now, just take the first command
                    if let Some(lite_cmd) = lite_pipeline.commands.first() {
                        if lite_cmd.args.len() != 2 {
                            return (
                                garbage(lite_arg.span),
                                Some(ParseError::mismatch("block", lite_arg.clone())),
                            );
                        }
                        let (lhs, err) =
                            parse_arg(SyntaxShape::FullColumnPath, registry, &lite_cmd.name);
                        if error.is_none() {
                            error = err;
                        }
                        let (op, err) =
                            parse_arg(SyntaxShape::Operator, registry, &lite_cmd.args[0]);
                        if error.is_none() {
                            error = err;
                        }
                        let (rhs, err) = parse_arg(SyntaxShape::Any, registry, &lite_cmd.args[1]);
                        if error.is_none() {
                            error = err;
                        }

                        let span = Span::new(lhs.span.start(), rhs.span.end());
                        let binary = SpannedExpression::new(
                            Expression::Binary(Box::new(Binary::new(lhs, op, rhs))),
                            span,
                        );
                        (
                            SpannedExpression::new(Expression::Block(vec![binary]), span),
                            error,
                        )
                    } else {
                        (
                            garbage(lite_arg.span),
                            Some(ParseError::mismatch("block", lite_arg.clone())),
                        )
                    }
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

fn classify_positional_arg(
    idx: usize,
    lite_cmd: &LiteCommand,
    positional_type: &PositionalType,
    registry: &dyn SignatureRegistry,
) -> (usize, SpannedExpression, Option<ParseError>) {
    let mut idx = idx;
    let mut error = None;
    let arg = match positional_type {
        PositionalType::Mandatory(_, SyntaxShape::Block)
        | PositionalType::Optional(_, SyntaxShape::Block) => {
            // We may have an implied block, so let's try to parse it here
            // The only implied block format we currently support is <shorthand path> <operator> <any>, though
            // we may want to revisit this in the future

            // TODO: only do this step if it's not a literal block
            if (idx + 2) < lite_cmd.args.len() {
                let (lhs, err) =
                    parse_arg(SyntaxShape::FullColumnPath, registry, &lite_cmd.args[idx]);
                if error.is_none() {
                    error = err;
                }
                let (op, err) = parse_arg(SyntaxShape::Operator, registry, &lite_cmd.args[idx + 1]);
                if error.is_none() {
                    error = err;
                }
                let (rhs, err) = parse_arg(SyntaxShape::Any, registry, &lite_cmd.args[idx + 2]);
                if error.is_none() {
                    error = err;
                }
                idx += 2;
                let span = Span::new(lhs.span.start(), rhs.span.end());
                let binary = SpannedExpression::new(
                    Expression::Binary(Box::new(Binary::new(lhs, op, rhs))),
                    span,
                );
                SpannedExpression::new(Expression::Block(vec![binary]), span)
            } else {
                let (arg, err) = parse_arg(SyntaxShape::Block, registry, &lite_cmd.args[idx]);
                if error.is_none() {
                    error = err;
                }
                arg
            }
        }
        PositionalType::Mandatory(_, shape) => {
            let (arg, err) = parse_arg(*shape, registry, &lite_cmd.args[idx]);
            if error.is_none() {
                error = err;
            }
            arg
        }
        PositionalType::Optional(_, shape) => {
            let (arg, err) = parse_arg(*shape, registry, &lite_cmd.args[idx]);
            if error.is_none() {
                error = err;
            }
            arg
        }
    };

    (idx, arg, error)
}

fn classify_internal_command(
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
                let (new_idx, expr, err) = classify_positional_arg(
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

    for lite_cmd in lite_pipeline.commands.iter() {
        if lite_cmd.name.item.starts_with('^') {
            let cmd_name: String = lite_cmd.name.item.chars().skip(1).collect();
            // This is an external command we should allow arguments to pass through with minimal parsing
            commands.push(ClassifiedCommand::External(ExternalCommand {
                name: cmd_name,
                name_tag: Tag::unknown_anchor(lite_cmd.name.span),
                args: ExternalArgs {
                    list: lite_cmd
                        .args
                        .iter()
                        .map(|x| ExternalArg {
                            arg: x.item.clone(),
                            tag: Tag::unknown_anchor(x.span),
                        })
                        .collect(),
                    span: Span::new(0, 0),
                },
            }))
        } else if let Some(signature) = registry.get(&lite_cmd.name.item) {
            let (internal_command, err) =
                classify_internal_command(&lite_cmd, registry, &signature);

            if error.is_none() {
                error = err;
            }
            commands.push(ClassifiedCommand::Internal(internal_command))
        } else {
            let trimmed = trim_quotes(&lite_cmd.name.item);
            let name = shellexpand::tilde(&trimmed).to_string();
            // This is an external command we should allow arguments to pass through with minimal parsing
            commands.push(ClassifiedCommand::External(ExternalCommand {
                name,
                name_tag: Tag::unknown_anchor(lite_cmd.name.span),
                args: ExternalArgs {
                    list: lite_cmd
                        .args
                        .iter()
                        .map(|x| ExternalArg {
                            arg: x.item.clone(),
                            tag: Tag::unknown_anchor(x.span),
                        })
                        .collect(),
                    span: Span::new(0, 0),
                },
            }))
        }
    }

    ClassifiedPipeline::new(commands, error)
}

/// Easy shorthand function to create a garbage expression at the given span
pub fn garbage(span: Span) -> SpannedExpression {
    SpannedExpression::new(Expression::Garbage, span)
}
