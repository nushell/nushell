use crate::{
    lex_once::lex_span,
    parse_block,
    parse_helpers::{garbage_pipeline, trim_quotes},
    parse_patterns::check_binding_type,
    parser::{
        ArgumentParsingLevel, ParsedInternalCall, parse_internal_call, parse_var_with_opt_type,
    },
    type_check::{check_pipeline_type, type_compatible},
};

use log::trace;
use nu_protocol::{
    ParseError, Span, Type, Value, VarId,
    ast::{Argument, Call, Expr, Expression, MatchPattern, Pipeline},
    engine::StateWorkingSet,
    eval_const::eval_constant,
};
use std::{collections::HashMap, sync::Arc};

// TODO: handle pipeline input type based inference
pub fn parse_let(
    working_set: &mut StateWorkingSet,
    spans: &[Span],
    input_type: Option<&Type>,
) -> Pipeline {
    trace!("parsing: let");

    if let Some(decl_id) = working_set.find_decl(b"let") {
        if spans.len() >= 4 {
            for span in spans.iter().enumerate() {
                let item = working_set.get_span_contents(*span.1);
                if item == b"=" && spans.len() > (span.0 + 1) && span.0 > 1 {
                    let (tokens, parse_error) = lex_span(
                        working_set,
                        Span::concat(&spans[(span.0 + 1)..]),
                        &[],
                        &[],
                        false,
                    );

                    if let Some(parse_error) = parse_error {
                        working_set.error(parse_error)
                    }

                    let rvalue_span = Span::concat(&spans[(span.0 + 1)..]);
                    let rvalue_block =
                        parse_block(working_set, &tokens, rvalue_span, false, true, input_type);

                    let output_type = match rvalue_block.pipelines.as_slice() {
                        [pipeline] if let Some(input) = input_type => {
                            match check_pipeline_type(working_set, pipeline, input) {
                                Ok(output_ty) => output_ty,
                                Err((output_ty, errors)) => {
                                    working_set.parse_errors.extend(errors);
                                    output_ty
                                }
                            }
                        }
                        _ => rvalue_block.output_type(),
                    };

                    let block_id = working_set.add_block(Arc::new(rvalue_block));

                    let rvalue = Expression::new(
                        working_set,
                        Expr::Block(block_id),
                        rvalue_span,
                        output_type,
                    );

                    let mut idx = 0;
                    let (lvalue, explicit_type) = parse_var_with_opt_type(
                        working_set,
                        &spans[1..(span.0)],
                        &mut idx,
                        false,
                        input_type,
                    );
                    if idx + 1 < span.0 - 1 {
                        working_set.error(ParseError::ExtraTokens(spans[idx + 2]));
                    }

                    let var_id = lvalue.as_var();
                    let rhs_type = rvalue.ty.clone();
                    if let Expr::Pattern(pattern) = &lvalue.expr
                        && explicit_type.is_none()
                    {
                        check_binding_type(working_set, pattern, &rhs_type, rvalue_span);
                    }

                    if let Some(explicit_type) = &explicit_type
                        && !type_compatible(explicit_type, &rhs_type)
                    {
                        working_set.error(ParseError::TypeMismatch(
                            explicit_type.clone(),
                            rhs_type.clone(),
                            Span::concat(&spans[(span.0 + 1)..]),
                        ));
                    }

                    if let Some(var_id) = var_id
                        && explicit_type.is_none()
                    {
                        working_set.set_variable_type(var_id, rhs_type);
                    }

                    let call = Box::new(Call {
                        decl_id,
                        head: spans[0],
                        arguments: vec![Argument::Positional(lvalue), Argument::Positional(rvalue)],
                        parser_info: HashMap::new(),
                    });

                    return Pipeline::from_vec(vec![Expression::new(
                        working_set,
                        Expr::Call(call),
                        Span::concat(spans),
                        Type::Any,
                    )]);
                }
            }
        }
        let ParsedInternalCall { call, output, .. } = parse_internal_call(
            working_set,
            spans[0],
            &spans[1..],
            decl_id,
            ArgumentParsingLevel::Full,
            input_type,
        );

        return Pipeline::from_vec(vec![Expression::new(
            working_set,
            Expr::Call(call),
            Span::concat(spans),
            output,
        )]);
    } else {
        working_set.error(ParseError::UnknownState(
            "internal error: let or const statements not found in core language".into(),
            Span::concat(spans),
        ))
    }

    working_set.error(ParseError::UnknownState(
        "internal error: let or const statement unparsable".into(),
        Span::concat(spans),
    ));

    garbage_pipeline(working_set, spans)
}

/// Additionally returns the constants it declares, each with its name: the one variable, or each
/// variable of a destructuring pattern, as in `const {$a, $b} = {a: 1, b: 2}`.
pub fn parse_const(
    working_set: &mut StateWorkingSet,
    spans: &[Span],
) -> (Pipeline, Vec<(Vec<u8>, VarId)>) {
    trace!("parsing: const");

    if let Some(decl_id) = working_set.find_decl(b"const") {
        if spans.len() >= 4 {
            for span in spans.iter().enumerate() {
                let item = working_set.get_span_contents(*span.1);
                if item == b"=" && spans.len() > (span.0 + 1) && span.0 > 1 {
                    let rvalue_span = Span::concat(&spans[(span.0 + 1)..]);

                    let (rvalue_tokens, rvalue_error) =
                        lex_span(working_set, rvalue_span, &[], &[], false);
                    working_set.parse_errors.extend(rvalue_error);

                    trace!("parsing: const right-hand side subexpression");
                    let rvalue_block =
                        parse_block(working_set, &rvalue_tokens, rvalue_span, false, true, None);
                    let rvalue_ty = rvalue_block.output_type();
                    let rvalue_block_id = working_set.add_block(Arc::new(rvalue_block));
                    let rvalue = Expression::new(
                        working_set,
                        Expr::Subexpression(rvalue_block_id),
                        rvalue_span,
                        rvalue_ty,
                    );

                    let mut idx = 0;

                    let (lvalue, explicit_type) = parse_var_with_opt_type(
                        working_set,
                        &spans[1..(span.0)],
                        &mut idx,
                        false,
                        None,
                    );
                    if idx + 1 < span.0 - 1 {
                        working_set.error(ParseError::ExtraTokens(spans[idx + 2]));
                    }

                    let var_id = lvalue.as_var();
                    let rhs_type = rvalue.ty.clone();
                    if let Expr::Pattern(pattern) = &lvalue.expr
                        && explicit_type.is_none()
                    {
                        check_binding_type(working_set, pattern, &rhs_type, rvalue_span);
                    }

                    if let Some(explicit_type) = &explicit_type
                        && !type_compatible(explicit_type, &rhs_type)
                    {
                        working_set.error(ParseError::TypeMismatch(
                            explicit_type.clone(),
                            rhs_type.clone(),
                            Span::concat(&spans[(span.0 + 1)..]),
                        ));
                    }

                    if let Some(var_id) = var_id {
                        if explicit_type.is_none() {
                            working_set.set_variable_type(var_id, rhs_type);
                        }

                        match eval_constant(working_set, &rvalue) {
                            Ok(mut value) => {
                                let mut const_type = value.get_type();

                                if let Some(explicit_type) = &explicit_type {
                                    if !type_compatible(explicit_type, &const_type) {
                                        working_set.error(ParseError::TypeMismatch(
                                            explicit_type.clone(),
                                            const_type.clone(),
                                            Span::concat(&spans[(span.0 + 1)..]),
                                        ));
                                    }
                                    let val_span = value.span();

                                    match value {
                                        nu_protocol::Value::String { val, .. }
                                            if explicit_type == &nu_protocol::Type::Glob =>
                                        {
                                            value = nu_protocol::Value::glob(val, false, val_span);
                                            const_type = value.get_type();
                                        }
                                        _ => {}
                                    }
                                }

                                working_set.set_variable_type(var_id, const_type);

                                working_set.set_variable_const_val(var_id, value);
                            }
                            Err(err) => working_set.error(err.wrap(working_set, rvalue.span)),
                        }
                    }

                    let constants = match &lvalue.expr {
                        Expr::Pattern(pattern) => {
                            bind_const_pattern(working_set, pattern, &rvalue, &explicit_type);
                            pattern
                                .variables()
                                .into_iter()
                                .map(|var_id| {
                                    let span = working_set.get_variable(var_id).declaration_span;
                                    let name = working_set.get_span_contents(span);
                                    (name.strip_prefix(b"$").unwrap_or(name).to_vec(), var_id)
                                })
                                .collect()
                        }
                        _ => var_id
                            .map(|var_id| {
                                let name = working_set.get_span_contents(lvalue.span);
                                (trim_quotes(name).to_vec(), var_id)
                            })
                            .into_iter()
                            .collect(),
                    };

                    let call = Box::new(Call {
                        decl_id,
                        head: spans[0],
                        arguments: vec![
                            Argument::Positional(lvalue.clone()),
                            Argument::Positional(rvalue),
                        ],
                        parser_info: HashMap::new(),
                    });

                    return (
                        Pipeline::from_vec(vec![Expression::new(
                            working_set,
                            Expr::Call(call),
                            Span::concat(spans),
                            Type::Any,
                        )]),
                        constants,
                    );
                }
            }
        }
        let ParsedInternalCall { call, output, .. } = parse_internal_call(
            working_set,
            spans[0],
            &spans[1..],
            decl_id,
            ArgumentParsingLevel::Full,
            None,
        );

        return (
            Pipeline::from_vec(vec![Expression::new(
                working_set,
                Expr::Call(call),
                Span::concat(spans),
                output,
            )]),
            vec![],
        );
    } else {
        working_set.error(ParseError::UnknownState(
            "internal error: let or const statements not found in core language".into(),
            Span::concat(spans),
        ))
    }

    working_set.error(ParseError::UnknownState(
        "internal error: let or const statement unparsable".into(),
        Span::concat(spans),
    ));

    (garbage_pipeline(working_set, spans), vec![])
}

/// Evaluate the value of a destructuring `const` and give each variable of its pattern a
/// constant value. `annotation` is the type written on the pattern, if any.
fn bind_const_pattern(
    working_set: &mut StateWorkingSet,
    pattern: &MatchPattern,
    rvalue: &Expression,
    annotation: &Option<Type>,
) {
    let value = match eval_constant(working_set, rvalue) {
        Ok(value) => value,
        Err(err) => return working_set.error(err.wrap(working_set, rvalue.span)),
    };
    if let Some(annotation) = annotation
        && !type_compatible(annotation, &value.get_type())
    {
        return working_set.error(ParseError::TypeMismatch(
            annotation.clone(),
            value.get_type(),
            rvalue.span,
        ));
    }
    let mut matches = vec![];
    if let Err(mismatch) = pattern.check(&value, &mut matches) {
        return working_set.error(mismatch.into_parse_error());
    }
    for (var_id, value) in matches {
        // A string bound to a variable annotated `glob` becomes a glob, as in `const g: glob`.
        let value = match value {
            Value::String {
                val, internal_span, ..
            } if working_set.get_variable(var_id).ty == Type::Glob => {
                Value::glob(val, false, internal_span)
            }
            value => value,
        };
        working_set.set_variable_type(var_id, value.get_type());
        working_set.set_variable_const_val(var_id, value);
    }
}

pub fn parse_mut(working_set: &mut StateWorkingSet, spans: &[Span]) -> Pipeline {
    trace!("parsing: mut");

    if let Some(decl_id) = working_set.find_decl(b"mut") {
        if spans.len() >= 4 {
            for span in spans.iter().enumerate() {
                let item = working_set.get_span_contents(*span.1);
                if item == b"=" && spans.len() > (span.0 + 1) && span.0 > 1 {
                    let (tokens, parse_error) = lex_span(
                        working_set,
                        Span::concat(&spans[(span.0 + 1)..]),
                        &[],
                        &[],
                        false,
                    );

                    if let Some(parse_error) = parse_error {
                        working_set.error(parse_error);
                    }

                    let rvalue_span = Span::concat(&spans[(span.0 + 1)..]);
                    let rvalue_block =
                        parse_block(working_set, &tokens, rvalue_span, false, true, None);

                    let output_type = rvalue_block.output_type();

                    let block_id = working_set.add_block(Arc::new(rvalue_block));

                    let rvalue = Expression::new(
                        working_set,
                        Expr::Block(block_id),
                        rvalue_span,
                        output_type,
                    );

                    let mut idx = 0;

                    let (lvalue, explicit_type) = parse_var_with_opt_type(
                        working_set,
                        &spans[1..(span.0)],
                        &mut idx,
                        true,
                        None,
                    );
                    if idx + 1 < span.0 - 1 {
                        working_set.error(ParseError::ExtraTokens(spans[idx + 2]));
                    }

                    let var_id = lvalue.as_var();
                    let rhs_type = rvalue.ty.clone();
                    if let Expr::Pattern(pattern) = &lvalue.expr
                        && explicit_type.is_none()
                    {
                        check_binding_type(working_set, pattern, &rhs_type, rvalue_span);
                    }

                    if let Some(explicit_type) = &explicit_type
                        && !type_compatible(explicit_type, &rhs_type)
                    {
                        working_set.error(ParseError::TypeMismatch(
                            explicit_type.clone(),
                            rhs_type.clone(),
                            Span::concat(&spans[(span.0 + 1)..]),
                        ));
                    }

                    if let Some(var_id) = var_id
                        && explicit_type.is_none()
                    {
                        working_set.set_variable_type(var_id, rhs_type);
                    }

                    let call = Box::new(Call {
                        decl_id,
                        head: spans[0],
                        arguments: vec![Argument::Positional(lvalue), Argument::Positional(rvalue)],
                        parser_info: HashMap::new(),
                    });

                    return Pipeline::from_vec(vec![Expression::new(
                        working_set,
                        Expr::Call(call),
                        Span::concat(spans),
                        Type::Any,
                    )]);
                }
            }
        }
        let ParsedInternalCall { call, output, .. } = parse_internal_call(
            working_set,
            spans[0],
            &spans[1..],
            decl_id,
            ArgumentParsingLevel::Full,
            None,
        );

        return Pipeline::from_vec(vec![Expression::new(
            working_set,
            Expr::Call(call),
            Span::concat(spans),
            output,
        )]);
    } else {
        working_set.error(ParseError::UnknownState(
            "internal error: let or const statements not found in core language".into(),
            Span::concat(spans),
        ))
    }

    working_set.error(ParseError::UnknownState(
        "internal error: let or const statement unparsable".into(),
        Span::concat(spans),
    ));

    garbage_pipeline(working_set, spans)
}
