//! Signatures, parameters and their types (the classic parser's `parse_signature_helper` and
//! `parse_full_signature`), and the predeclaration of commands (`parse_def_predecl`).

use nu_protocol::{
    Flag, ParseError, PositionalArg, Signature, SyntaxShape, Type, TypeSet, VarId,
    ast::{Expr, Expression},
    engine::CommandType,
    eval_const::eval_constant,
};
use nu_winnow_parser::{Span as WSpan, ast as w};

use super::{Lower, Lowered, Unlowered};
use crate::{
    parse_keywords::reject_parser_keyword_name,
    parse_shape_specs::{parse_completer, parse_shape_name},
    parse_signatures::{ensure_not_reserved_variable_name, parse_input_output_types},
};

/// A parameter as it is read, before the signature is assembled.
enum Parameter {
    Positional { arg: PositionalArg, required: bool },
    Rest(PositionalArg),
    Flag { flag: Flag, type_annotated: bool },
}

/// What the predeclaration of a command needs.
pub(in crate::winnow) struct Predecl<'d, 's> {
    /// The command's name, unquoted, and the span of its item.
    pub(in crate::winnow) name: &'d str,
    pub(in crate::winnow) name_span: WSpan,
    /// The signature, when the statement has one that parses.
    pub(in crate::winnow) signature: Option<&'d w::Signature<'s>>,
    /// Whether the definition has the `--wrapped` flag.
    pub(in crate::winnow) wrapped: bool,
    /// Whether it is an `extern`.
    pub(in crate::winnow) external: bool,
    /// Where the statement ends: the classic predeclaration reads the input and output types
    /// from everything after the signature, a `def`'s body included.
    pub(in crate::winnow) statement_end: usize,
}

impl<'s> Lower<'_, '_, 's> {
    /// Declare a command before the statements of its block are lowered, so calls to it
    /// resolve (`parse_def_predecl`). Returns `false` when the lowering cannot read the
    /// signature, for the classic predeclaration to do it.
    pub(in crate::winnow) fn predecl(&mut self, predecl: Predecl<'_, 's>) -> bool {
        // Without a signature the winnow parser could read (none, or one with an error), the
        // classic predeclaration decides, name checks included.
        let Some(signature) = predecl.signature else {
            return false;
        };
        // A name item in brackets or parentheses (`def [foo] ...`): the classic predeclaration
        // reads the signature from it, or gives up.
        if self.text(predecl.name_span).starts_with(['[', '(']) {
            return false;
        }
        let name = predecl.name;
        let name_span = self.span(predecl.name_span);
        if name.contains('#')
            || name.contains('^')
            || name.contains('%')
            || name.parse::<bytesize::ByteSize>().is_ok()
            || name.parse::<f64>().is_ok()
        {
            self.working_set
                .error(ParseError::CommandDefNotValid(name_span));
            return true;
        }
        if reject_parser_keyword_name(self.working_set, name, "command", name_span) {
            return true;
        }
        let errors = self.working_set.parse_errors.len();
        self.working_set.enter_scope();
        let lowered = self.signature_params(signature, predecl.external);
        let lowered = lowered.map(|mut sig| {
            if let Some(types) = signature.input_output_span {
                let span = self.span(WSpan::new(types.start, predecl.statement_end));
                sig.input_output_types = parse_input_output_types(self.working_set, &[span]);
            }
            sig
        });
        self.working_set.parse_errors.truncate(errors);
        self.working_set.exit_scope();
        let Ok(mut signature) = lowered else {
            return false;
        };
        signature.name = name.to_string();
        if predecl.wrapped {
            if let Some(rest) = &mut signature.rest_positional
                && !self.rest_is_typed(predecl.signature, &rest.name)
            {
                rest.shape = SyntaxShape::ExternalArgument;
            }
            signature.allows_unknown_args = true;
        }
        let command_type = if predecl.external {
            CommandType::External
        } else {
            CommandType::Custom
        };
        let decl = signature.predeclare_with_command_type(command_type);
        if self.working_set.add_predecl(decl).is_some() {
            self.working_set
                .error(ParseError::DuplicateCommandDef(name_span));
        }
        true
    }

    /// Declare the command a `def` or `extern` statement of a nested block defines.
    pub(super) fn predecl_statement(&mut self, e: &w::Expression<'s>) -> Lowered<()> {
        let (name, signature, wrapped, external) = match &e.expr {
            w::Expr::Def(def) => (
                &def.name,
                &def.signature,
                def.flags
                    .iter()
                    .any(|flag| flag.item == w::DefFlag::Wrapped),
                false,
            ),
            w::Expr::Extern(extern_def) => (&extern_def.name, &extern_def.signature, false, true),
            w::Expr::Export(export) => return self.predecl_statement(&export.item),
            w::Expr::AttributeBlock(block) => return self.predecl_statement(&block.item),
            _ => return Ok(()),
        };
        let lowered = self.predecl(Predecl {
            name: &name.item,
            name_span: name.span,
            signature: Some(signature),
            wrapped,
            external,
            statement_end: e.span.end,
        });
        if lowered {
            Ok(())
        } else {
            Err(Unlowered::Unsupported("predeclaration"))
        }
    }

    /// Whether the rest parameter `name` has a type in the signature's text
    /// (`rest_param_is_type_annotated`).
    fn rest_is_typed(&self, signature: Option<&w::Signature<'s>>, name: &str) -> bool {
        signature.is_some_and(|signature| {
            signature.params.iter().any(|param| {
                matches!(param.kind, w::ParameterKind::Rest)
                    && param.name.item == name
                    && param.ty.is_some()
            })
        })
    }

    /// A signature with its input and output types, as an expression (`parse_full_signature`).
    pub(super) fn signature_expression(
        &mut self,
        signature: &w::Signature<'s>,
        is_external: bool,
    ) -> Lowered<Expression> {
        let sig = self.full_signature(signature, is_external)?;
        let span = self.span(signature.span);
        Ok(self.node(Expr::Signature(sig), span, Type::Any))
    }

    /// A signature with its input and output types.
    fn full_signature(
        &mut self,
        signature: &w::Signature<'s>,
        is_external: bool,
    ) -> Lowered<Box<Signature>> {
        let mut sig = self.signature_params(signature, is_external)?;
        if let Some(types) = signature.input_output_span {
            let span = self.span(types);
            sig.input_output_types =
                self.checked(|working_set| parse_input_output_types(working_set, &[span]))?;
        }
        Ok(sig)
    }

    /// The parameters of a signature (`parse_signature_helper`). The parameters of a command
    /// get variables, declared once every parameter is read so that default values refer to
    /// variables outside the signature; an `extern`'s get none.
    pub(super) fn signature_params(
        &mut self,
        signature: &w::Signature<'s>,
        is_external: bool,
    ) -> Lowered<Box<Signature>> {
        let mut parameters: Vec<Parameter> = Vec::with_capacity(signature.params.len());
        let mut pending: Vec<(Vec<u8>, VarId)> = Vec::new();
        for param in &signature.params {
            let token = self.span(parameter_token(param));
            let (var_name, initial_ty) = match &param.kind {
                w::ParameterKind::Flag {
                    long: Some(long), ..
                } => (long.item.replace('-', "_").into_bytes(), Type::Bool),
                w::ParameterKind::Flag {
                    short: Some(short), ..
                } => (short.item.to_string().into_bytes(), Type::Bool),
                w::ParameterKind::Flag { .. } => return Err(Unlowered::Error),
                _ => (param.name.item.as_bytes().to_vec(), Type::Any),
            };
            let var_id = if is_external {
                None
            } else {
                self.checked(|working_set| {
                    ensure_not_reserved_variable_name(working_set, &var_name, token)
                })?;
                let var_id = self
                    .working_set
                    .add_variable_without_scope(token, initial_ty, false);
                pending.push((var_name, var_id));
                Some(var_id)
            };

            let (shape, completion) = match &param.ty {
                Some(ty) => {
                    let span = self.span(ty.span);
                    let text = self.text(ty.span).as_bytes();
                    let shape =
                        self.checked(|working_set| parse_shape_name(working_set, text, span))?;
                    let completion = match &param.completer {
                        Some(completer) => {
                            let span = self.span(completer.span);
                            let text = completer.item.as_bytes();
                            self.checked(|working_set| parse_completer(working_set, text, span))?
                        }
                        None => None,
                    };
                    (Some(shape), completion)
                }
                None => (None, None),
            };
            let type_annotated = shape.is_some();
            let desc = self.parameter_description(param);

            let mut parameter = match &param.kind {
                w::ParameterKind::Flag { long, short } => {
                    if shape == Some(SyntaxShape::Boolean) {
                        return Err(Unlowered::Error);
                    }
                    if let (Some(var_id), Some(shape)) = (var_id, &shape) {
                        self.working_set.set_variable_type(var_id, shape.to_type());
                    }
                    Parameter::Flag {
                        flag: Flag {
                            arg: shape.clone(),
                            desc,
                            long: long.map(|long| long.item.to_string()).unwrap_or_default(),
                            short: short.map(|short| short.item),
                            required: false,
                            var_id,
                            default_value: None,
                            completion,
                        },
                        type_annotated,
                    }
                }
                w::ParameterKind::Rest => {
                    let shape = shape.clone().unwrap_or(SyntaxShape::Any);
                    if let (Some(var_id), true) = (var_id, type_annotated) {
                        self.working_set
                            .set_variable_type(var_id, Type::List(Box::new(shape.to_type())));
                    }
                    Parameter::Rest(PositionalArg {
                        name: param.name.item.to_string(),
                        desc,
                        shape,
                        var_id,
                        default_value: None,
                        completion,
                    })
                }
                w::ParameterKind::Required | w::ParameterKind::Optional => {
                    let arg_shape = shape.clone().unwrap_or(SyntaxShape::Any);
                    if let (Some(var_id), true) = (var_id, type_annotated) {
                        self.working_set
                            .set_variable_type(var_id, arg_shape.to_type());
                    }
                    Parameter::Positional {
                        arg: PositionalArg {
                            name: param.name.item.to_string(),
                            desc,
                            shape: arg_shape,
                            var_id,
                            default_value: None,
                            completion,
                        },
                        required: matches!(param.kind, w::ParameterKind::Required),
                    }
                }
            };

            if let (Some(default), false) = (&param.default, is_external) {
                self.default_value(&mut parameter, default, type_annotated)?;
            }
            parameters.push(parameter);
        }

        for (name, var_id) in pending {
            self.working_set.insert_variable_into_scope(name, var_id);
        }

        let mut sig = Signature::new(String::new());
        for parameter in parameters {
            match parameter {
                Parameter::Positional { arg, required } => {
                    if required {
                        if !sig.optional_positional.is_empty() {
                            return Err(Unlowered::Error);
                        }
                        sig.required_positional.push(arg);
                    } else {
                        // An optional parameter without a default can be `null`.
                        if arg.default_value.is_none()
                            && let Some(var_id) = arg.var_id
                        {
                            let ty = self
                                .working_set
                                .get_variable(var_id)
                                .ty
                                .clone()
                                .union(Type::Nothing);
                            self.working_set.set_variable_type(var_id, ty);
                        }
                        sig.optional_positional.push(arg);
                    }
                }
                Parameter::Flag {
                    flag,
                    type_annotated,
                } => {
                    if type_annotated
                        && flag.default_value.is_none()
                        && let Some(var_id) = flag.var_id
                    {
                        let ty = self
                            .working_set
                            .get_variable(var_id)
                            .ty
                            .clone()
                            .union(Type::Nothing);
                        self.working_set.set_variable_type(var_id, ty);
                    }
                    sig.named.push(flag);
                }
                Parameter::Rest(arg) => {
                    if arg.name.is_empty() || sig.rest_positional.is_some() {
                        return Err(Unlowered::Error);
                    }
                    sig.rest_positional = Some(arg);
                }
            }
        }
        Ok(Box::new(sig))
    }

    /// A parameter's default value: a constant, whose type is the parameter's when it has
    /// none.
    fn default_value(
        &mut self,
        parameter: &mut Parameter,
        default: &w::Expression<'s>,
        type_annotated: bool,
    ) -> Lowered<()> {
        let shape = match parameter {
            Parameter::Positional { arg, .. } => arg.shape.clone(),
            Parameter::Rest(_) => return Err(Unlowered::Error),
            Parameter::Flag { flag, .. } => flag.arg.clone().unwrap_or(SyntaxShape::Any),
        };
        let expression = self.value(default, &shape, None)?;
        let value = eval_constant(self.working_set, &expression).map_err(|_| Unlowered::Error)?;
        match parameter {
            Parameter::Positional { arg, required } => {
                if let Some(var_id) = arg.var_id
                    && self.working_set.get_variable(var_id).ty == Type::Any
                    && !type_annotated
                {
                    self.working_set
                        .set_variable_type(var_id, expression.ty.clone());
                }
                arg.default_value = Some(value);
                if !type_annotated {
                    arg.shape = expression.ty.to_shape();
                }
                *required = false;
            }
            Parameter::Flag { flag, .. } => {
                flag.default_value = Some(value);
                if !type_annotated {
                    flag.arg = Some(expression.ty.to_shape());
                    if let Some(var_id) = flag.var_id {
                        self.working_set
                            .set_variable_type(var_id, expression.ty.clone());
                    }
                }
            }
            Parameter::Rest(_) => return Err(Unlowered::Error),
        }
        Ok(())
    }

    /// A parameter's description: the comments after it, each without its `#`, one per line.
    fn parameter_description(&self, param: &w::Parameter<'s>) -> String {
        let mut desc = String::new();
        for comment in &param.description {
            let text = self
                .text(WSpan::new(comment.span.start + 1, comment.span.end))
                .trim();
            if !desc.is_empty() {
                desc.push('\n');
            }
            desc.push_str(text);
        }
        desc
    }
}

/// The item of a parameter as the classic lexer splits it: `x`, `x?`, `...x`, `--flag`,
/// `--flag(-f)` or `-f`, which its variable's span covers.
fn parameter_token(param: &w::Parameter<'_>) -> WSpan {
    let name = param.name.span;
    match &param.kind {
        w::ParameterKind::Required => name,
        w::ParameterKind::Optional => WSpan::new(name.start, name.end + 1),
        w::ParameterKind::Rest => WSpan::new(name.start - 3, name.end),
        w::ParameterKind::Flag {
            long: Some(long),
            short,
        } => {
            let start = long.span.start - 2;
            match short {
                // `--flag(-f)` is one item; `--flag (-f)` two.
                Some(short) if short.span.start == long.span.end + 2 => {
                    WSpan::new(start, short.span.end + 1)
                }
                _ => WSpan::new(start, long.span.end),
            }
        }
        w::ParameterKind::Flag {
            long: None,
            short: Some(short),
        } => WSpan::new(short.span.start - 1, short.span.end),
        w::ParameterKind::Flag { .. } => param.span,
    }
}
