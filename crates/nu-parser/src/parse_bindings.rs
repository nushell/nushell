use crate::{
    lex, parse_block,
    parse_helpers::garbage_pipeline,
    parser::{
        ArgumentParsingLevel, ParsedInternalCall, parse_internal_call, parse_var_with_opt_type,
    },
    type_check::type_compatible,
};

use log::trace;
use nu_protocol::{
    ParseError, Span, Type,
    ast::{Argument, Call, Expr, Expression, Pipeline},
    engine::StateWorkingSet,
    eval_const::eval_constant,
};
use std::{collections::HashMap, sync::Arc};

// TODO: handle pipeline input type based inference
pub fn parse_let(working_set: &mut StateWorkingSet, spans: &[Span]) -> Pipeline {
    trace!("parsing: let");

    if let Some(decl_id) = working_set.find_decl(b"let") {
        if spans.len() >= 4 {
            for span in spans.iter().enumerate() {
                let item = working_set.get_span_contents(*span.1);
                if item == b"=" && spans.len() > (span.0 + 1) && span.0 > 1 {
                    let (tokens, parse_error) = lex(
                        working_set.get_span_contents(Span::concat(&spans[(span.0 + 1)..])),
                        spans[span.0 + 1].start,
                        &[],
                        &[],
                        false,
                    );

                    if let Some(parse_error) = parse_error {
                        working_set.error(parse_error)
                    }

                    let rvalue_span = Span::concat(&spans[(span.0 + 1)..]);
                    let rvalue_block = parse_block(working_set, &tokens, rvalue_span, false, true);

                    let output_type = rvalue_block.output_type();

                    let block_id = working_set.add_block(Arc::new(rvalue_block));

                    let rvalue = Expression::new(
                        working_set,
                        Expr::Block(block_id),
                        rvalue_span,
                        output_type,
                    );

                    let mut idx = 0;
                    let (lvalue, explicit_type) =
                        parse_var_with_opt_type(working_set, &spans[1..(span.0)], &mut idx, false);
                    if idx + 1 < span.0 - 1 {
                        working_set.error(ParseError::ExtraTokens(spans[idx + 2]));
                    }

                    let var_id = lvalue.as_var();
                    let rhs_type = rvalue.ty.clone();

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

/// Additionally returns a span encompassing the variable name, if successful.
pub fn parse_const(working_set: &mut StateWorkingSet, spans: &[Span]) -> (Pipeline, Option<Span>) {
    trace!("parsing: const");

    if let Some(decl_id) = working_set.find_decl(b"const") {
        if spans.len() >= 4 {
            for span in spans.iter().enumerate() {
                let item = working_set.get_span_contents(*span.1);
                if item == b"=" && spans.len() > (span.0 + 1) && span.0 > 1 {
                    let rvalue_span = Span::concat(&spans[(span.0 + 1)..]);

                    let (rvalue_tokens, rvalue_error) = lex(
                        working_set.get_span_contents(rvalue_span),
                        rvalue_span.start,
                        &[],
                        &[],
                        false,
                    );
                    working_set.parse_errors.extend(rvalue_error);

                    trace!("parsing: const right-hand side subexpression");
                    let rvalue_block =
                        parse_block(working_set, &rvalue_tokens, rvalue_span, false, true);
                    let rvalue_ty = rvalue_block.output_type();
                    let rvalue_block_id = working_set.add_block(Arc::new(rvalue_block));
                    let rvalue = Expression::new(
                        working_set,
                        Expr::Subexpression(rvalue_block_id),
                        rvalue_span,
                        rvalue_ty,
                    );

                    let mut idx = 0;

                    let (lvalue, explicit_type) =
                        parse_var_with_opt_type(working_set, &spans[1..(span.0)], &mut idx, false);
                    if idx + 1 < span.0 - 1 {
                        working_set.error(ParseError::ExtraTokens(spans[idx + 2]));
                    }

                    let var_id = lvalue.as_var();
                    let rhs_type = rvalue.ty.clone();

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
                        Some(lvalue.span),
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
            None,
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

    (garbage_pipeline(working_set, spans), None)
}

pub fn parse_mut(working_set: &mut StateWorkingSet, spans: &[Span]) -> Pipeline {
    trace!("parsing: mut");

    if let Some(decl_id) = working_set.find_decl(b"mut") {
        if spans.len() >= 4 {
            for span in spans.iter().enumerate() {
                let item = working_set.get_span_contents(*span.1);
                if item == b"=" && spans.len() > (span.0 + 1) && span.0 > 1 {
                    let (tokens, parse_error) = lex(
                        working_set.get_span_contents(Span::concat(&spans[(span.0 + 1)..])),
                        spans[span.0 + 1].start,
                        &[],
                        &[],
                        false,
                    );

                    if let Some(parse_error) = parse_error {
                        working_set.error(parse_error);
                    }

                    let rvalue_span = Span::concat(&spans[(span.0 + 1)..]);
                    let rvalue_block = parse_block(working_set, &tokens, rvalue_span, false, true);

                    let output_type = rvalue_block.output_type();

                    let block_id = working_set.add_block(Arc::new(rvalue_block));

                    let rvalue = Expression::new(
                        working_set,
                        Expr::Block(block_id),
                        rvalue_span,
                        output_type,
                    );

                    let mut idx = 0;

                    let (lvalue, explicit_type) =
                        parse_var_with_opt_type(working_set, &spans[1..(span.0)], &mut idx, true);
                    if idx + 1 < span.0 - 1 {
                        working_set.error(ParseError::ExtraTokens(spans[idx + 2]));
                    }

                    let var_id = lvalue.as_var();
                    let rhs_type = rvalue.ty.clone();

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
