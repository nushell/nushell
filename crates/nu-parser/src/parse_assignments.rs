use crate::{
    eval::eval_constant_assignment,
    lex, lex_signature, parse_block,
    parser::{garbage_pipeline, is_variable, parse_multispan_value, parse_shape_name},
    type_check::type_compatible,
};
use nu_protocol::{
    ast::{Argument, Call, Expr, Expression, Pipeline},
    engine::StateWorkingSet,
    span as mk_span, ParseError, Span, Spanned, SyntaxShape, Type,
};

#[derive(Debug)]
pub struct Assignment {
    tokens: Vec<Span>,
    name: Spanned<String>,
    kind: Spanned<Kind>,
    typ: Option<Type>,
    val_start: usize,
}

impl Assignment {
    pub fn try_parse(working_set: &mut StateWorkingSet, spans: &[Span]) -> Option<Self> {
        try_parse(working_set, spans)
    }

    pub fn process(self, working_set: &mut StateWorkingSet) -> Pipeline {
        process(working_set, self)
    }
}

fn process(working_set: &mut StateWorkingSet, asg: Assignment) -> Pipeline {
    let spans = &asg.tokens;
    let decl_id = {
        let kw = asg.kind.item.as_bytes();
        let Some(id) = working_set.find_decl(kw, &Type::Nothing) else {
            working_set.error(ParseError::UnknownState(
                format!("internal error: {} statement not found in core language", asg.kind.item),
                mk_span(spans),
            ));

            return garbage_pipeline(spans);
        };

        id
    };

    let mk_var = |working_set: &mut StateWorkingSet<'_>, ty: Type| -> (Argument, usize) {
        let mutable = asg.kind.item.is_mutable();
        let (name, span) = (asg.name.item.as_bytes(), asg.name.span);
        let ty = asg.typ.clone().unwrap_or(ty);
        let id = working_set.add_variable(name.to_vec(), span, ty.clone(), mutable);
        let expr = Expression {
            expr: Expr::VarDecl(id),
            span,
            ty,
            custom_completion: None,
        };
        (Argument::Positional(expr), id)
    };

    let (name, val) = {
        let val_spans = &spans[asg.val_start..];
        let span = mk_span(val_spans);
        let val_tokens = {
            let bytes = working_set.get_span_contents(span);
            let (tokens, err) = lex(bytes, span.start, &[], &[], true);
            if let Some(err) = err {
                working_set.error(err);
            }
            tokens
        };

        let chk_ty = |lhs, rhs, span| -> Option<ParseError> {
            if type_compatible(lhs, rhs) {
                None
            } else {
                Some(ParseError::TypeMismatch(lhs.clone(), rhs.clone(), span))
            }
        };

        match asg.kind.item {
            Kind::Const => {
                let val = parse_multispan_value(
                    working_set,
                    &val_tokens.iter().map(|x| x.span).collect::<Vec<_>>(),
                    &mut 0,
                    &SyntaxShape::MathExpression,
                );

                if let Some(err) = chk_ty(&asg.typ.clone().unwrap_or(Type::Any), &val.ty, val.span)
                {
                    working_set.error(err);
                }

                let (name, var_id) = mk_var(working_set, val.ty.clone());

                match eval_constant_assignment(working_set, &val) {
                    Ok(val) => working_set.add_constant(var_id, val),
                    Err(err) => working_set.error(err),
                }

                (name, Argument::Positional(val))
            }

            _ => {
                let block = parse_block(working_set, &val_tokens, span, false, true);

                if let Some(err) = chk_ty(
                    &asg.typ.clone().unwrap_or(Type::Any),
                    &block.output_type(),
                    span,
                ) {
                    working_set.error(err);
                }

                let (name, _) = mk_var(working_set, block.output_type());

                let ty = block.output_type();
                let id = working_set.add_block(block);
                let val = Expression {
                    expr: Expr::Block(id),
                    span,
                    ty,
                    custom_completion: None,
                };

                (name, Argument::Positional(val))
            }
        }
    };

    let call = Box::new(Call {
        decl_id,
        head: asg.kind.span,
        arguments: vec![name, val],
        redirect_stdout: true,
        redirect_stderr: false,
        parser_info: std::collections::HashMap::new(),
    });

    Pipeline::from_vec(vec![Expression {
        expr: Expr::Call(call),
        span: mk_span(spans),
        ty: Type::Any,
        custom_completion: None,
    }])
}

fn try_parse(working_set: &mut StateWorkingSet, spans: &[Span]) -> Option<Assignment> {
    let span = mk_span(spans);
    let input = working_set.get_span_contents(span);
    let (tokens, error) = lex_signature(input, span.start, &[], &[b':', b'='], false);
    if let Some(error) = error {
        working_set.error(error);
    }

    let Some(kw_token) = tokens.first() else {
        working_set.error(ParseError::InternalError("not an assignment".into(), span));
        return None;
    };

    let kind = Kind::from(working_set.get_span_contents(kw_token.span));

    let Some(name_token) = tokens.get(1) else {
        let end = kw_token.span.end - 1;
        let err = ParseError::Expected("a variable name", Span::new(end, end));
        working_set.error(err);
        return None;
    };

    let name_bytes = working_set.get_span_contents(name_token.span);

    if name_bytes == b"=" {
        let end = kw_token.span.end - 1;
        let err = ParseError::Expected("a variable name", Span::new(end, end));
        working_set.error(err);
        return None;
    }

    if name_bytes.contains(&b' ')
        || name_bytes.contains(&b'"')
        || name_bytes.contains(&b'\'')
        || name_bytes.contains(&b'`')
        || !is_variable(name_bytes)
    {
        working_set.error(ParseError::VariableNotValid(name_token.span));
        return None;
    }

    let name = String::from_utf8_lossy(name_bytes)
        .trim_start_matches('$')
        .to_string();

    if ["in", "nu", "env", "nothing"].contains(&name.as_str()) {
        working_set.error(ParseError::NameIsBuiltinVar(name, name_token.span));
        return None;
    }

    let Some(colon_or_eq) = tokens.get(2) else {
        let end = name_token.span.end - 1;
        let span = Span::new(end, end);
        working_set.error(ParseError::Expected("expected `:` or `=` after the name", span));
        return None;
    };

    let shape = if working_set.get_span_contents(colon_or_eq.span) == b":" {
        let Some(ty_token) = tokens.get(3) else {
            let end = colon_or_eq.span.end - 1;
            let span = Span::new(end, end);
            working_set.error(ParseError::Expected("expected a type after the colon", span));
            return None;
        };

        let bytes = working_set.get_span_contents(ty_token.span).to_vec();
        Some(parse_shape_name(working_set, &bytes, ty_token.span))
    } else {
        None
    };

    let (eq_token_pos, eq_error) = if shape.is_some() {
        let end = tokens[3].span.end - 1;
        let span = Span::new(end, end);
        let err = ParseError::Expected("expected an `=` after the type", span);
        (4, err)
    } else {
        let end = name_token.span.end - 1;
        let span = Span::new(end, end);
        let err = ParseError::Expected("expected an `=` after the name", span);
        (2, err)
    };

    let Some(eq_token) = tokens.get(eq_token_pos) else {
        working_set.error(eq_error);
        return None;
    };

    if working_set.get_span_contents(eq_token.span) != b"=" {
        working_set.error(eq_error);
        return None;
    }

    let val_start = eq_token_pos + 1;
    if tokens.get(val_start).is_none() {
        let end = eq_token.span.end - 1;
        let span = Span::new(end, end);
        working_set.error(ParseError::Expected("a value after `=`", span));
        None
    } else {
        Some(Assignment {
            tokens: tokens.iter().map(|x| x.span).collect(),
            name: Spanned {
                item: name,
                span: name_token.span,
            },
            kind: Spanned {
                item: kind,
                span: kw_token.span,
            },
            typ: shape.map(|s| s.to_type()),
            val_start,
        })
    }
}

#[derive(Clone, Copy, Debug)]
enum Kind {
    Let,
    Mut,
    Const,
}

impl Kind {
    const fn is_mutable(&self) -> bool {
        matches!(self, Kind::Mut)
    }

    const fn as_bytes<'kind>(&self) -> &'kind [u8] {
        match self {
            Kind::Let => b"let",
            Kind::Mut => b"mut",
            Kind::Const => b"const",
        }
    }
}

impl std::fmt::Display for Kind {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Kind::Let => f.write_str("let"),
            Kind::Mut => f.write_str("mut"),
            Kind::Const => f.write_str("const"),
        }
    }
}

impl From<&[u8]> for Kind {
    fn from(kw: &[u8]) -> Self {
        match kw {
            b"let" => Kind::Let,
            b"mut" => Kind::Mut,
            b"const" => Kind::Const,
            _ => unreachable!(
                "internal error: {} is not an assignment",
                String::from_utf8_lossy(kw).to_string()
            ),
        }
    }
}
