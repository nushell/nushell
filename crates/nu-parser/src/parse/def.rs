use crate::parse::{classify_block, parse_arg, util::trim_quotes};

use indexmap::IndexMap;
use nu_errors::ParseError;
use nu_protocol::{
    hir::{Block, Expression, Literal, SpannedExpression},
    NamedType,
};
use nu_protocol::{PositionalType, Signature, SyntaxShape};
use nu_source::{Spanned, SpannedItem};

//use crate::errors::{ParseError, ParseResult};
use crate::lex::{block, lex, LiteCommand};

use crate::ParserScope;

fn parse_type(type_: &str, signature_vec: &Spanned<String>) -> (SyntaxShape, Option<ParseError>) {
    match type_ {
        "int" => (SyntaxShape::Int, None),
        "string" => (SyntaxShape::String, None),
        "path" => (SyntaxShape::FilePath, None),
        "table" => (SyntaxShape::Table, None),
        "unit" => (SyntaxShape::Unit, None),
        "number" => (SyntaxShape::Number, None),
        "pattern" => (SyntaxShape::GlobPattern, None),
        "range" => (SyntaxShape::Range, None),
        "block" => (SyntaxShape::Block, None),
        "any" => (SyntaxShape::Any, None),
        _ => (
            SyntaxShape::Any,
            Some(ParseError::mismatch(
                "params with known types",
                signature_vec.clone(),
            )),
        ),
    }
}

pub(crate) fn parse_definition(call: &LiteCommand, scope: &dyn ParserScope) -> Option<ParseError> {
    // A this point, we've already handled the prototype and put it into scope;
    // So our main goal here is to parse the block now that the names and
    // prototypes of adjacent commands are also available

    if call.parts.len() == 4 {
        if call.parts.len() != 4 {
            return Some(ParseError::mismatch("definition", call.parts[0].clone()));
        }

        if call.parts[0].item != "def" {
            return Some(ParseError::mismatch("definition", call.parts[0].clone()));
        }

        let name = trim_quotes(&call.parts[1].item);
        let (mut signature, err) = parse_signature(&name, &call.parts[2], scope);

        //Add commands comments to signature usage
        signature.usage = call.comments_joined();

        if err.is_some() {
            return err;
        };

        let mut chars = call.parts[3].chars();
        match (chars.next(), chars.next_back()) {
            (Some('{'), Some('}')) => {
                // We have a literal block
                let string: String = chars.collect();

                scope.enter_scope();

                let (tokens, err) = lex(&string, call.parts[3].span.start() + 1);
                if err.is_some() {
                    return err;
                };
                let (lite_block, err) = block(tokens);
                if err.is_some() {
                    return err;
                };

                let (mut block, err) = classify_block(&lite_block, scope);
                scope.exit_scope();

                block.params = signature;
                block.params.name = name;

                scope.add_definition(block);

                err
            }
            _ => Some(ParseError::mismatch("body", call.parts[3].clone())),
        }
    } else {
        Some(ParseError::internal_error(
            "need a block".to_string().spanned(call.span()),
        ))
    }
}

pub(crate) fn parse_definition_prototype(
    call: &LiteCommand,
    scope: &dyn ParserScope,
) -> Option<ParseError> {
    let mut err = None;

    if call.parts.len() != 4 {
        return Some(ParseError::mismatch("definition", call.parts[0].clone()));
    }

    if call.parts[0].item != "def" {
        return Some(ParseError::mismatch("definition", call.parts[0].clone()));
    }

    let name = trim_quotes(&call.parts[1].item);
    let (signature, error) = parse_signature(&name, &call.parts[2], scope);
    if err.is_none() {
        err = error;
    }

    scope.add_definition(Block::new(signature, vec![], IndexMap::new(), call.span()));

    err
}

fn parse_signature(
    name: &str,
    signature_vec: &Spanned<String>,
    scope: &dyn ParserScope,
) -> (Signature, Option<ParseError>) {
    let mut err = None;

    let (preparsed_params, error) = parse_arg(SyntaxShape::Table, scope, signature_vec);
    if err.is_none() {
        err = error;
    }
    let mut signature = Signature::new(name);

    if let SpannedExpression {
        expr: Expression::List(preparsed_params),
        ..
    } = preparsed_params
    {
        for preparsed_param in preparsed_params.iter() {
            match &preparsed_param.expr {
                Expression::Literal(Literal::String(st)) => {
                    let parts: Vec<_> = st.split(':').collect();
                    if parts.len() == 1 {
                        if parts[0].starts_with("--") {
                            // Flag
                            let flagname = parts[0][2..].to_string();
                            signature
                                .named
                                .insert(flagname, (NamedType::Switch(None), String::new()));
                        } else {
                            // Positional
                            signature.positional.push((
                                PositionalType::Mandatory(parts[0].to_string(), SyntaxShape::Any),
                                String::new(),
                            ));
                        }
                    } else if parts.len() == 2 {
                        if parts[0].starts_with("--") {
                            // Flag
                            let flagname = parts[0][2..].to_string();
                            let (shape, parse_type_err) = parse_type(parts[1], signature_vec);
                            if err.is_none() {
                                err = parse_type_err;
                            }

                            signature.named.insert(
                                flagname,
                                (NamedType::Optional(None, shape), String::new()),
                            );
                        } else {
                            // Positional
                            let name = parts[0].to_string();
                            let (shape, parse_type_err) = parse_type(parts[1], signature_vec);
                            if err.is_none() {
                                err = parse_type_err;
                            }
                            signature
                                .positional
                                .push((PositionalType::Mandatory(name, shape), String::new()));
                        }
                    } else if err.is_none() {
                        err = Some(ParseError::mismatch(
                            "param with type",
                            signature_vec.clone(),
                        ));
                    }
                }
                _ => {
                    if err.is_none() {
                        err = Some(ParseError::mismatch("parameter", signature_vec.clone()));
                    }
                }
            }
        }
        (signature, err)
    } else {
        (
            signature,
            Some(ParseError::mismatch("parameters", signature_vec.clone())),
        )
    }
}
