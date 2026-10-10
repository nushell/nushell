//! Calls: internal commands with their arguments assigned by signature, aliases, and external
//! commands (the classic parser's `parse_call`, `parse_internal_call` and
//! `parse_external_call`).

use nu_engine::DIR_VAR_PARSER_INFO;
use nu_protocol::{
    DeclId, Flag, Span, Spanned, SyntaxShape, Type,
    ast::{Call, Expr, Expression, ExternalArgument},
};
use nu_winnow_parser::{DeclKind, ast as w};

use super::{Lower, Lowered, Unlowered};
use crate::{
    parse_calls::{
        CallKind, check_call, find_longest_decl_with_prefix, parse_external_string,
        parse_regular_external_arg, parse_unknown_arg, shape_allows_negative_number,
    },
    parse_expressions::parse_value as classic_value,
    parse_keywords::is_parser_keyword,
    parse_source::{LIB_DIRS_VAR, find_dirs_var},
    type_check::type_compatible,
    winnow::lookup::decl_kind,
};

/// The commands the classic parser picks by a statement's first word, before it looks up any
/// command name (`parse_builtin_commands`; `run` also after a `|`, in `parse_expression`): a
/// longer command name that starts with one of them is never called there.
const FIRST_WORD_COMMANDS: &[&str] = &["hide", "run", "source", "source-env"];

impl<'s> Lower<'_, '_, 's> {
    /// A call to a command (`parse_call`).
    pub(super) fn call(
        &mut self,
        e: &w::Expression<'s>,
        call: &w::Call<'s>,
        input_type: Option<&Type>,
    ) -> Lowered<Expression> {
        if call.sigil.is_some() {
            return Err(Unlowered::Unsupported("`%` call"));
        }
        let head_span = self.span(call.head.span);
        let Some(decl_id) = self.working_set.find_decl(call.head.name.as_bytes()) else {
            return Err(Unlowered::Unsupported("unresolved command"));
        };
        // The winnow parser read the arguments as external arguments, for a `def --wrapped`
        // command; the classic parser reads them with the signature of the command the name
        // resolves to now (`parse_internal_call`), which a `hide`, `use` or `overlay use` may
        // have changed since.
        if call.wrapped && decl_kind(self.working_set, decl_id) != DeclKind::Wrapped {
            return Err(Unlowered::Unsupported("wrapped command rebound"));
        }
        let decl = self.working_set.get_decl(decl_id);
        // A parser keyword has a parse function of its own in the classic parser
        // (`parse_builtin_commands`), which applies its effects on the working set. Left to the
        // lowering are the control-flow keywords, which the winnow tree has nodes for, and
        // `overlay` alone, a plain call.
        if decl.is_keyword()
            && is_parser_keyword(decl.name().as_bytes())
            && !matches!(
                decl.name(),
                "if" | "match"
                    | "try"
                    | "overlay"
                    | "loop"
                    | "while"
                    | "return"
                    | "break"
                    | "continue"
            )
        {
            return Err(Unlowered::Unsupported("keyword command"));
        }
        if call
            .head
            .name
            .split(' ')
            .next()
            .is_some_and(|word| FIRST_WORD_COMMANDS.contains(&word))
        {
            return Err(Unlowered::Unsupported("keyword command"));
        }
        // `let = 1` and friends: the classic parser reports an incomplete statement.
        if let Some(w::Argument::Positional(first)) = call.arguments.first()
            && self.text(first.span) == "="
        {
            return Err(Unlowered::Error);
        }
        let span = self.span(e.span);
        if let Some(alias) = decl.as_alias()
            && let Expr::ExternalCall(head, args) = &alias.wrapped_call.expr
        {
            // An alias of an external command: its arguments come first.
            let ty = alias.wrapped_call.ty.clone();
            let mut head = head.clone();
            head.span = head_span;
            let mut arguments = args.to_vec();
            for argument in &call.arguments {
                arguments.push(self.call_argument_as_external(argument)?);
            }
            return Ok(self.node(Expr::ExternalCall(head, arguments.into()), span, ty));
        }
        // The words after an alias of a command may name a subcommand of that command
        // (`alias u = update; u cells {..}`), which the classic parser then calls instead
        // (`find_longest_decl_with_prefix`, asked here with the aliased command's name).
        if let Some(alias) = decl.as_alias()
            && let Expr::Call(wrapped) = &alias.wrapped_call.expr
            && !call.arguments.is_empty()
        {
            let prefix = self
                .working_set
                .get_decl(wrapped.decl_id)
                .name()
                .as_bytes()
                .to_vec();
            let spans: Vec<Span> = call
                .arguments
                .iter()
                .map(|argument| self.span(argument.span()))
                .collect();
            if find_longest_decl_with_prefix(self.working_set, &spans, &prefix)
                .3
                .is_some()
            {
                return Err(Unlowered::Unsupported(
                    "alias of a command with a subcommand",
                ));
            }
        }
        let (call, output) = self.internal_call(head_span, &call.arguments, decl_id, input_type)?;
        Ok(self.node(Expr::Call(call), span, output))
    }

    /// A call to `decl_id` with its arguments assigned by the command's signature
    /// (`parse_internal_call`): flags and their values, positionals read with their shapes and
    /// type-checked, spreads, and the unknown arguments of a command that takes them. Returns
    /// the call and its output type for `input_type`.
    pub(super) fn internal_call(
        &mut self,
        head: Span,
        arguments: &[w::Argument<'s>],
        decl_id: DeclId,
        input_type: Option<&Type>,
    ) -> Lowered<(Box<Call>, Type)> {
        let mut call = Call::new(head);
        call.decl_id = decl_id;
        // Registered as `parse_internal_call` registers it.
        let _ = self.working_set.add_span(head);

        let decl = self.working_set.get_decl(decl_id);
        let signature = self.working_set.get_signature_shared(decl_id);
        let deprecation = decl.deprecation_info();
        let checks_lib_dirs = decl.name() == "nu-check" && decl.is_builtin();
        let alias_call = decl.as_alias().map(|alias| alias.wrapped_call.clone());

        let output = self
            .working_set
            .call_output_type(decl_id, &signature, input_type)
            .unwrap_or(Type::Error);

        let mut positional_idx = 0;
        if let Some(wrapped) = alias_call {
            // An alias of an internal call: its call with the alias's name, and its positionals
            // already given.
            let Expr::Call(wrapped) = wrapped.expr else {
                return Err(Unlowered::Error);
            };
            call = *wrapped;
            call.head = head;
            positional_idx = call.positional_iter().count();
        }
        // `parse_internal_call` hands the library directories to `nu-check` and to `use`,
        // `overlay use` and `source-env`, which never get here (parser keywords, which
        // `Lower::call` leaves to the classic parser).
        if checks_lib_dirs && let Some(var_id) = find_dirs_var(self.working_set, LIB_DIRS_VAR) {
            let var = self.node(Expr::Var(var_id), call.head, Type::Any);
            call.set_parser_info(DIR_VAR_PARSER_INFO.to_owned(), var);
        }

        if signature.creates_scope {
            self.working_set.enter_scope();
        }
        let result = self.assign_arguments(&mut call, arguments, &signature, positional_idx);
        if signature.creates_scope {
            self.working_set.exit_scope();
        }
        result?;

        let call_kind =
            self.checked(|working_set| check_call(working_set, head, &signature, &call))?;
        if call_kind == CallKind::Invalid {
            return Err(Unlowered::Error);
        }
        for warning in deprecation
            .into_iter()
            .filter_map(|entry| entry.parse_warning(&signature.name, &call))
        {
            self.working_set.warning(warning);
        }
        Ok((Box::new(call), output))
    }

    /// Put `arguments` into `call` as the signature says, in source order.
    fn assign_arguments(
        &mut self,
        call: &mut Call,
        arguments: &[w::Argument<'s>],
        signature: &nu_protocol::Signature,
        mut positional_idx: usize,
    ) -> Lowered<()> {
        let mut end_of_options = false;
        // An index rather than an iterator: a flag may take the next argument as its value.
        let mut index = 0;
        while let Some(argument) = arguments.get(index) {
            index += 1;
            match argument {
                w::Argument::EndOfOptions(span) if !end_of_options => {
                    end_of_options = true;
                    if signature.allows_unknown_args {
                        let span = self.span(*span);
                        let value = self.checked(|ws| parse_unknown_arg(ws, span, signature))?;
                        call.add_unknown(value);
                    }
                }
                w::Argument::Named(flag) if !end_of_options => {
                    // `-inf` or `-nan`, which `parse_short_flags` reads as a number here.
                    if !flag.long
                        && is_negative_number(self.text(flag.span), signature, positional_idx)
                    {
                        return Err(Unlowered::Unsupported("`-inf` or `-nan` as a number"));
                    }
                    let taken = self.flag(call, flag, arguments.get(index), signature)?;
                    if taken {
                        index += 1;
                    }
                }
                // After `--`, a flag or another `--` is an ordinary positional.
                w::Argument::Named(w::NamedArgument { span, .. })
                | w::Argument::EndOfOptions(span) => {
                    let span = self.span(*span);
                    if let Some(positional) = signature.get_positional(positional_idx) {
                        let shape = positional.shape.clone();
                        let value = self.checked(|ws| classic_value(ws, span, &shape, None))?;
                        if !type_compatible(&shape.to_type(), &value.ty) {
                            return Err(Unlowered::Error);
                        }
                        call.add_positional(value);
                        positional_idx += 1;
                    } else if signature.allows_unknown_args {
                        let value = self.checked(|ws| parse_unknown_arg(ws, span, signature))?;
                        call.add_unknown(value);
                    } else {
                        return Err(Unlowered::Error);
                    }
                }
                w::Argument::Spread { expr, .. } => {
                    positional_idx = self.spread(call, expr, signature, positional_idx)?;
                }
                w::Argument::Positional(expr) => {
                    // Before `--`, an item starting with `-` is short flags unless it is a
                    // negative number for the next positional (`parse_short_flags`).
                    let text = self.text(expr.span);
                    if !end_of_options
                        && text.len() > 1
                        && text.starts_with('-')
                        && !is_negative_number(text, signature, positional_idx)
                    {
                        return Err(Unlowered::Error);
                    }
                    if let Some(positional) = signature.get_positional(positional_idx) {
                        let shape = &positional.shape;
                        // A row condition reads the items from here on (`parse_multispan_value`):
                        // a closure that is the call's last item is one by itself
                        // (`parse_row_condition`).
                        let value = if matches!(shape, SyntaxShape::RowCondition)
                            && index == arguments.len()
                            && matches!(expr.expr, w::Expr::Closure(_))
                        {
                            self.row_condition(expr)?
                        } else if is_multispan_shape(shape) {
                            return Err(Unlowered::Unsupported("multi-item argument"));
                        } else {
                            self.value(expr, shape, None)?
                        };
                        if !type_compatible(&shape.to_type(), &value.ty) {
                            return Err(Unlowered::Error);
                        }
                        call.add_positional(value);
                        positional_idx += 1;
                    } else if signature.allows_unknown_args {
                        let value = self.value(expr, &rest_shape(signature), None)?;
                        call.add_unknown(value);
                    } else {
                        return Err(Unlowered::Error);
                    }
                }
            }
        }
        Ok(())
    }

    /// A long or short flag (`parse_long_flag`, `parse_short_flags`). Returns whether it took
    /// `next`, a positional argument, as its value.
    fn flag(
        &mut self,
        call: &mut Call,
        flag: &w::NamedArgument<'s>,
        next: Option<&w::Argument<'s>>,
        signature: &nu_protocol::Signature,
    ) -> Lowered<bool> {
        let token = self.span(flag.span);
        if flag.long {
            let Some(found) = signature.get_long_flag(flag.name) else {
                return self.unknown_flag(call, token, signature).map(|()| false);
            };
            // `--name=value` names the flag with `--name`; `--name value` with the whole item.
            let name_span = match &flag.value {
                Some(_) => Span::new(token.start, token.start + flag.name.len() + 2),
                None => token,
            };
            let name = Spanned {
                item: found.long.clone(),
                span: name_span,
            };
            let (value, taken) = match (&found.arg, &flag.value) {
                (Some(shape), Some(value)) => (Some(self.flag_value(value, shape)?), false),
                (Some(shape), None) => match next {
                    Some(w::Argument::Positional(value)) => {
                        (Some(self.flag_value(value, shape)?), true)
                    }
                    _ => return Err(Unlowered::Error),
                },
                (None, Some(value)) => {
                    (Some(self.flag_value(value, &SyntaxShape::Boolean)?), false)
                }
                (None, None) => (None, false),
            };
            call.add_named((name, None, value));
            return Ok(taken);
        }
        if flag.value.is_some() {
            return Err(Unlowered::Unsupported("short flag with `=`"));
        }
        let mut found: Vec<Flag> = Vec::new();
        let count = flag.name.chars().count();
        for (offset, short) in flag.name.chars().enumerate() {
            match signature.get_short_flag(short) {
                Some(short_flag) if short_flag.arg.is_some() && offset + 1 < count => {
                    return Err(Unlowered::Error);
                }
                Some(short_flag) => found.push(short_flag),
                None => return self.unknown_flag(call, token, signature).map(|()| false),
            }
        }
        let mut taken = false;
        for short_flag in found {
            let short = short_flag.short.map(|short| Spanned {
                item: short.to_string(),
                span: token,
            });
            let value = match &short_flag.arg {
                Some(shape) => match next {
                    Some(w::Argument::Positional(value)) => {
                        taken = true;
                        Some(self.flag_value(value, shape)?)
                    }
                    _ => return Err(Unlowered::Error),
                },
                None => None,
            };
            let name = Spanned {
                item: short_flag.long.clone(),
                span: token,
            };
            if short_flag.long.is_empty() {
                call.add_named((name, short, value));
            } else {
                call.add_named((name, None, value));
            }
        }
        Ok(taken)
    }

    /// A flag's value, type-checked against its shape (`ensure_flag_arg_type`); `null` passes.
    fn flag_value(
        &mut self,
        value: &w::Expression<'s>,
        shape: &SyntaxShape,
    ) -> Lowered<Expression> {
        let value = self.value(value, shape, None)?;
        if value.ty != Type::Nothing && !type_compatible(&shape.to_type(), &value.ty) {
            return Err(Unlowered::Error);
        }
        Ok(value)
    }

    /// A flag the signature does not have: an unknown argument when the command takes them
    /// (`extern`, `def --wrapped`), an error otherwise.
    fn unknown_flag(
        &mut self,
        call: &mut Call,
        token: Span,
        signature: &nu_protocol::Signature,
    ) -> Lowered<()> {
        if !signature.allows_unknown_args {
            return Err(Unlowered::Error);
        }
        let value = self.checked(|ws| parse_unknown_arg(ws, token, signature))?;
        call.add_unknown(value);
        Ok(())
    }

    /// A spread argument, `...$list`, `...[a b]` or `...{flag: value}`. Returns the index of
    /// the next positional: past them all once the spread fills the rest parameter, unchanged
    /// when it is, or may be, a record of flags.
    fn spread(
        &mut self,
        call: &mut Call,
        expr: &w::Expression<'s>,
        signature: &nu_protocol::Signature,
        positional_idx: usize,
    ) -> Lowered<usize> {
        let can_rest_spread = signature.rest_positional.is_some() || signature.allows_unknown_args;
        let can_named_spread = signature.named.iter().any(|named| named.long != "help");
        let required = signature.required_positional.len();
        let all_positionals = required + signature.optional_positional.len();
        match self.text(expr.span).as_bytes().first() {
            Some(b'{') => {
                if !can_named_spread {
                    return Err(Unlowered::Error);
                }
                let value = self.value(
                    expr,
                    &SyntaxShape::Record(std::iter::empty().collect()),
                    None,
                )?;
                call.add_spread(value);
                Ok(positional_idx)
            }
            Some(b'[') => {
                if !can_rest_spread || positional_idx < required {
                    return Err(Unlowered::Error);
                }
                let shape = match &signature.rest_positional {
                    Some(rest) if rest.shape != SyntaxShape::ExternalArgument => rest.shape.clone(),
                    _ => SyntaxShape::Any,
                };
                let value = self.value(expr, &SyntaxShape::List(Box::new(shape)), None)?;
                call.add_spread(value);
                Ok(all_positionals)
            }
            Some(b'$' | b'(') => {
                // Without named flags to spread, a spread fills the rest parameter, which
                // must exist and cannot stand in for a missing required positional.
                if !can_named_spread && (!can_rest_spread || positional_idx < required) {
                    return Err(Unlowered::Error);
                }
                let value = self.value(expr, &SyntaxShape::Any, None)?;
                call.add_spread(value);
                if can_rest_spread && !can_named_spread {
                    Ok(all_positionals)
                } else {
                    Ok(positional_idx)
                }
            }
            _ => Err(Unlowered::Error),
        }
    }

    /// A call to an external command (`parse_external_call`), or to an alias of one, which
    /// the winnow parser also makes an external call.
    pub(super) fn external_call(
        &mut self,
        e: &w::Expression<'s>,
        call: &w::ExternalCall<'s>,
    ) -> Lowered<Expression> {
        if call.caret.is_none()
            && let w::Expr::String(name) = &call.head.expr
            && name.quote == w::Quote::Bare
            && let Some(decl_id) = self.working_set.find_decl(name.value.as_bytes())
        {
            // `alias "run tests" = ^echo`: the classic parser picks `run` by the first word.
            if name
                .value
                .split(' ')
                .next()
                .is_some_and(|word| FIRST_WORD_COMMANDS.contains(&word))
            {
                return Err(Unlowered::Unsupported("keyword command"));
            }
            // A known name here is an alias of an external command; any other command resolved
            // differently for the winnow parser, which made an external call, so the classic
            // parser decides.
            let Some(alias) = self.working_set.get_decl(decl_id).as_alias() else {
                return Err(Unlowered::Unsupported("external call to a known command"));
            };
            let Expr::ExternalCall(alias_head, alias_args) = &alias.wrapped_call.expr else {
                return Err(Unlowered::Unsupported("external call to an alias"));
            };
            let ty = alias.wrapped_call.ty.clone();
            let mut head = alias_head.clone();
            head.span = self.span(call.head.span);
            let mut arguments = alias_args.to_vec();
            for argument in &call.arguments {
                arguments.push(self.external_argument_node(argument)?);
            }
            let span = self.span(e.span);
            return Ok(self.node(Expr::ExternalCall(head, arguments.into()), span, ty));
        }
        // Any other head is a string or glob, even `[a b]` or `{a}` (`parse_external_string`).
        let head = match self.text(call.head.span).as_bytes().first() {
            Some(b'$' | b'(') => self.expression(&call.head, None)?,
            _ => {
                let span = self.span(call.head.span);
                self.checked(|ws| parse_external_string(ws, span))?
            }
        };
        let mut arguments = Vec::with_capacity(call.arguments.len());
        for argument in &call.arguments {
            arguments.push(self.external_argument_node(argument)?);
        }
        let span = self.span(e.span);
        Ok(self.node(
            Expr::ExternalCall(Box::new(head), arguments.into()),
            span,
            Type::Any,
        ))
    }

    /// An argument of an external command (`parse_external_arg`).
    fn external_argument_node(
        &mut self,
        argument: &w::ExternalArgument<'s>,
    ) -> Lowered<ExternalArgument> {
        Ok(match argument {
            w::ExternalArgument::Regular(value) => {
                ExternalArgument::Regular(self.external_argument(value)?)
            }
            w::ExternalArgument::Spread { expr, .. } => ExternalArgument::Spread(self.value(
                expr,
                &SyntaxShape::List(Box::new(SyntaxShape::Any)),
                None,
            )?),
        })
    }

    /// An argument of an external command (`parse_regular_external_arg`): a structured value
    /// read with any shape, a word as a glob or string with its quotes kept track of.
    fn external_argument(&mut self, value: &w::Expression<'s>) -> Lowered<Expression> {
        match self.text(value.span).as_bytes().first() {
            Some(b'$' | b'(' | b'{') => self.value(value, &SyntaxShape::Any, None),
            Some(b'[') => match &value.expr {
                w::Expr::List(items) => self.list(value.span, items, &SyntaxShape::Any),
                _ => Err(Unlowered::Unsupported("external list argument")),
            },
            _ => {
                let span = self.span(value.span);
                self.checked(|ws| parse_regular_external_arg(ws, span))
            }
        }
    }

    /// An argument of a call to an alias of an external command, read as an external argument.
    fn call_argument_as_external(
        &mut self,
        argument: &w::Argument<'s>,
    ) -> Lowered<ExternalArgument> {
        match argument {
            w::Argument::Positional(value) => {
                Ok(ExternalArgument::Regular(self.external_argument(value)?))
            }
            w::Argument::Named(flag) => {
                let span = self.span(flag.span);
                Ok(ExternalArgument::Regular(
                    self.checked(|ws| parse_regular_external_arg(ws, span))?,
                ))
            }
            w::Argument::EndOfOptions(span) => {
                let span = self.span(*span);
                Ok(ExternalArgument::Regular(
                    self.checked(|ws| parse_regular_external_arg(ws, span))?,
                ))
            }
            w::Argument::Spread { expr, .. } => Ok(ExternalArgument::Spread(self.value(
                expr,
                &SyntaxShape::List(Box::new(SyntaxShape::Any)),
                None,
            )?)),
        }
    }
}

/// The shape of a command's rest parameter, which its unknown arguments are read with.
fn rest_shape(signature: &nu_protocol::Signature) -> SyntaxShape {
    signature
        .rest_positional
        .as_ref()
        .map_or(SyntaxShape::Any, |rest| rest.shape.clone())
}

/// `parse_short_flags`' rule for an item starting with `-`, before `--`: it is a negative number
/// for the positional at `positional_idx` when none of its characters is a short flag of
/// `signature`, the positional takes a number (`shape_allows_negative_number`) and the item
/// parses as one; otherwise it is short flags.
fn is_negative_number(
    text: &str,
    signature: &nu_protocol::Signature,
    positional_idx: usize,
) -> bool {
    text.strip_prefix('-').is_some_and(|flags| {
        !flags
            .chars()
            .any(|short| signature.get_short_flag(short).is_some())
    }) && signature
        .get_positional(positional_idx)
        .is_some_and(|positional| shape_allows_negative_number(&positional.shape))
        && text.parse::<f64>().is_ok()
}

/// Whether a positional's shape reads several items, which the lowering does not assign to
/// the winnow parser's single arguments.
fn is_multispan_shape(shape: &SyntaxShape) -> bool {
    match shape {
        SyntaxShape::MathExpression
        | SyntaxShape::Expression
        | SyntaxShape::RowCondition
        | SyntaxShape::Keyword(..)
        | SyntaxShape::VarWithOptType
        | SyntaxShape::Signature
        | SyntaxShape::ExternalSignature
        | SyntaxShape::ImportPattern => true,
        SyntaxShape::OneOf(shapes) => shapes.iter().any(is_multispan_shape),
        _ => false,
    }
}
