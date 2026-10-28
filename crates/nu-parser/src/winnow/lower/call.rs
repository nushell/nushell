//! Calls: internal commands with their arguments assigned by signature, aliases, and external
//! commands (the classic parser's `parse_call`, `parse_internal_call` and
//! `parse_external_call`).

use std::cell::RefCell;

use nu_engine::DIR_VAR_PARSER_INFO;
use nu_protocol::{
    CompileError, DeclId, Flag, Span, Spanned, SyntaxShape, Type, TypeSet,
    ast::{Argument, Call, Expr, Expression, ExternalArgument, Keyword},
    engine::CommandType,
};

use super::{Lower, Lowered, Unlowered};
use crate::winnow::syntax::{DeclKind, ItemsValue, Span as WSpan, ast as w, parse_items};
use crate::{
    parse_calls::{
        CallKind, accepts_any_bareword, find_decl_with_command_type, find_longest_decl_with_prefix,
        parse_external_string, parse_regular_external_arg, parse_unknown_arg,
        shape_allows_negative_number, strict_extern_forwards,
    },
    parse_expressions::{is_math_expression_like, parse_value as classic_value},
    parse_helpers::{PERCENT_FORCED_BUILTIN_PARSER_INFO, is_variable},
    parse_keywords::is_parser_keyword,
    parse_signatures::ensure_not_reserved_variable_name,
    parse_source::{LIB_DIRS_VAR, find_dirs_var},
    type_check::type_compatible,
    winnow::lookup::{EngineLookup, decl_kind},
};

/// The first words `parse_expression` reports a statement for where it reads a command (in a
/// pipeline, after an environment shorthand): `BuiltinCommandInPipeline` and
/// `AssignInPipeline`.
const STATEMENTS_IN_PIPELINE: &[&str] = &[
    "def",
    "extern",
    "for",
    "module",
    "type",
    "use",
    "source",
    "alias",
    "export",
    "export-env",
    "hide",
    "const",
    "mut",
];

impl<'s> Lower<'_, '_, 's> {
    /// A call to a command (`parse_call`).
    pub(super) fn call(
        &mut self,
        e: &w::Expression<'s>,
        call: &w::Call<'s>,
        input_type: Option<&Type>,
    ) -> Lowered<Expression> {
        if let Some(sigil) = call.sigil {
            return self.builtin_call(e, call, sigil, input_type);
        }
        let Some(decl_id) = self.working_set.find_decl(call.head.name.as_bytes()) else {
            // In an alias's body, the predeclared commands are hidden (`parse_alias`).
            if !self.working_set.search_predecls {
                return self.call_as_external(e, call);
            }
            return Err(Unlowered::Unsupported("unresolved command"));
        };
        if !self.alias_target {
            // Where the classic parser reads a command (`parse_expression`), its first word picks
            // how (`HeadKind`): `run` is `parse_run_expr`, a statement is an error, and
            // `overlay` but `overlay list`, and `plugin use`, are errors too (both looked for
            // in the second item). Statements are parsed before (`builtin_commands`).
            let first_word = call.head.name.split(' ').next().unwrap_or_default();
            match first_word {
                "run" if call.head.name == "run" => return self.run(e, call),
                "run" | "where" => return Err(Unlowered::Unsupported("keyword command")),
                word if STATEMENTS_IN_PIPELINE.contains(&word) => return Err(Unlowered::Error),
                "overlay" if self.second_word(call) != Some("list") => {
                    return Err(Unlowered::Error);
                }
                "plugin" if self.second_word(call) == Some("use") => return Err(Unlowered::Error),
                _ => {}
            }
            // Any other parser keyword with a parse function of its own the winnow tree has no
            // node for; `let` and `source-env` are ordinary calls there, and the control-flow
            // keywords have nodes.
            let decl = self.working_set.get_decl(decl_id);
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
                        | "let"
                        | "source-env"
                )
            {
                return Err(Unlowered::Unsupported("keyword command"));
            }
        }
        self.ordinary_call(e, call, decl_id, input_type)
    }

    /// The second item of `call`: the second word of its head, or else its first argument.
    fn second_word(&self, call: &w::Call<'s>) -> Option<&'s str> {
        match call.head.name.split(' ').nth(1) {
            Some(_) => {
                let head = self.text(call.head.span);
                head.split_ascii_whitespace().nth(1)
            }
            None => call
                .arguments
                .first()
                .map(|argument| self.text(argument.span())),
        }
    }

    /// A call to `decl_id`, which `call`'s head names, read as `parse_call` reads it: an alias of
    /// an external command takes external arguments, and any other command its signature's.
    pub(super) fn ordinary_call(
        &mut self,
        e: &w::Expression<'s>,
        call: &w::Call<'s>,
        decl_id: DeclId,
        input_type: Option<&Type>,
    ) -> Lowered<Expression> {
        let head_span = self.span(call.head.span);
        // The winnow parser read the arguments as external arguments, for a `def --wrapped`
        // command; the classic parser reads them with the signature of the command the name
        // resolves to now (`parse_internal_call`), which a `hide`, `use` or `overlay use` may
        // have changed since.
        if call.wrapped && decl_kind(self.working_set, decl_id) != DeclKind::Wrapped {
            return Err(Unlowered::Unsupported("wrapped command rebound"));
        }
        let decl = self.working_set.get_decl(decl_id);
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
        let (call, output, _) =
            self.internal_call(head_span, &call.arguments, decl_id, input_type)?;
        Ok(self.node(Expr::Call(call), span, output))
    }

    /// `%name args`, a call to the built-in command `name` even where a custom command or alias
    /// shadows or hides it (`parse_call` with the `%` sigil, `find_decl_with_command_type`). As
    /// there, the head starts past the `%`, so after a lone `%` (`% ls`) it covers the blank
    /// before the name.
    fn builtin_call(
        &mut self,
        e: &w::Expression<'s>,
        call: &w::Call<'s>,
        sigil: WSpan,
        input_type: Option<&Type>,
    ) -> Lowered<Expression> {
        let decl_id = find_decl_with_command_type(
            self.working_set,
            call.head.name.as_bytes(),
            CommandType::Builtin,
        )
        .ok_or(Unlowered::Error)?;
        let head = Span::new(self.span(sigil).end, self.span(call.head.span).end);
        let (lowered, output, _) =
            self.internal_call(head, &call.arguments, decl_id, input_type)?;
        let span = self.span(e.span);
        Ok(self.node(Expr::Call(lowered), span, output))
    }

    /// `%$cmd args` or `%(expr) args`, a call to the built-in command an expression names at
    /// run time (`parse_call`'s dynamic `%` dispatch): a placeholder call covering the whole
    /// call, its head included, with the head expression in its parser info, which the IR
    /// compiler makes a call to `run-internal`. Each argument is read with any shape, a flag or
    /// `--` as a word, and a spread as a list.
    pub(super) fn dynamic_call(
        &mut self,
        e: &w::Expression<'s>,
        call: &w::DynamicCall<'s>,
        input_type: Option<&Type>,
    ) -> Lowered<Expression> {
        let span = self.span(e.span);
        let head = self.expression(&call.head, input_type)?;
        let mut lowered = Call::new(span);
        lowered.decl_id = DeclId::new(0);
        lowered.set_parser_info(PERCENT_FORCED_BUILTIN_PARSER_INFO.to_string(), head);
        for argument in &call.arguments {
            let argument = match argument {
                w::Argument::Positional(expr) => {
                    Argument::Positional(self.value(expr, &SyntaxShape::Any, None)?)
                }
                w::Argument::Spread { expr, .. } => {
                    let shape = SyntaxShape::List(Box::new(SyntaxShape::Any));
                    Argument::Spread(self.value(expr, &shape, None)?)
                }
                w::Argument::Named(w::NamedArgument { span, .. })
                | w::Argument::EndOfOptions(span) => {
                    let span = self.span(*span);
                    Argument::Positional(
                        self.checked(|ws| classic_value(ws, span, &SyntaxShape::Any, None))?,
                    )
                }
            };
            lowered.arguments.push(argument);
        }
        Ok(self.node(Expr::Call(Box::new(lowered)), span, Type::Any))
    }

    /// A call to `decl_id` with its arguments assigned by the command's signature
    /// (`parse_internal_call`): flags and their values, positionals read with their shapes and
    /// type-checked, spreads, and the unknown arguments of a command that takes them. Returns
    /// the call, its output type for `input_type` and its kind (a call for help, or not).
    pub(super) fn internal_call(
        &mut self,
        head: Span,
        arguments: &[w::Argument<'s>],
        decl_id: DeclId,
        input_type: Option<&Type>,
    ) -> Lowered<(Box<Call>, Type, CallKind)> {
        let mut call = Call::new(head);
        call.decl_id = decl_id;
        // Registered as `parse_internal_call` registers it.
        let _ = self.working_set.add_span(head);

        let decl = self.working_set.get_decl(decl_id);
        let signature = self.working_set.get_signature_shared(decl_id);
        let deprecation = decl.deprecation_info();
        let checks_lib_dirs = match decl.name() {
            "use" | "overlay use" | "source-env" => decl.is_keyword(),
            "nu-check" => decl.is_builtin(),
            _ => false,
        };
        let alias_call = decl.as_alias().map(|alias| alias.wrapped_call.clone());
        let special = match decl.is_keyword().then(|| decl.name()) {
            Some("let") => Some(SpecialKeyword::Let),
            Some("match") => Some(SpecialKeyword::Match),
            Some("if") => Some(SpecialKeyword::If),
            _ => None,
        };

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
        // `overlay use` and `source-env`.
        if checks_lib_dirs && let Some(var_id) = find_dirs_var(self.working_set, LIB_DIRS_VAR) {
            let var = self.node(Expr::Var(var_id), call.head, Type::Any);
            call.set_parser_info(DIR_VAR_PARSER_INFO.to_owned(), var);
        }

        if signature.creates_scope {
            self.working_set.enter_scope();
        }
        let result = self.assign_arguments(
            &mut call,
            arguments,
            &signature,
            positional_idx,
            special,
            input_type,
        );
        if signature.creates_scope {
            self.working_set.exit_scope();
        }
        let mut output_override = result?;
        // Without an `else` (counting every argument, flags too), `if` may give nothing.
        if special == Some(SpecialKeyword::If) && call.arguments.len() < 3 {
            output_override = output_override.map(|ty| ty.union(Type::Nothing));
        }
        let output = output_override.unwrap_or(output);

        let kind = self.check_call(head, &signature, &call)?;
        for warning in deprecation
            .into_iter()
            .filter_map(|entry| entry.parse_warning(&signature.name, &call))
        {
            self.working_set.warning(warning);
        }
        Ok((Box::new(call), output, kind))
    }

    /// Put `arguments` into `call` as the signature says, in source order. `special` is the
    /// keyword `parse_internal_call` reads some positionals of in its own way (`SpecialCmd`):
    /// `let`'s variable, `if`'s blocks and `match`'s arms get `input_type`, and the call's output
    /// is the variable's input, the branches' union or the arms' type, which this returns.
    fn assign_arguments(
        &mut self,
        call: &mut Call,
        arguments: &[w::Argument<'s>],
        signature: &nu_protocol::Signature,
        mut positional_idx: usize,
        special: Option<SpecialKeyword>,
        input_type: Option<&Type>,
    ) -> Lowered<Option<Type>> {
        let mut output_override = None;
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
                w::Argument::Named(flag)
                    if !end_of_options
                        && (flag.long
                            || !is_negative_number(
                                self.text(flag.span),
                                signature,
                                positional_idx,
                            )) =>
                {
                    let taken = self.flag(call, flag, arguments.get(index), signature)?;
                    if taken {
                        index += 1;
                    }
                }
                // After `--`, a flag or another `--` is an ordinary positional, and so is
                // `-inf` or `-nan` for a positional that takes a number (`parse_short_flags`).
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
                    // negative number for the next positional (`parse_short_flags`): the
                    // command's own flags when it has them all (`def f [--one(-1)]; f -1`),
                    // else an unknown flag, which `extern` and `def --wrapped` commands take
                    // as an argument (`f -1` for `def --wrapped f [...rest]`).
                    let text = self.text(expr.span);
                    if !end_of_options
                        && text.len() > 1
                        && text.starts_with('-')
                        && !is_negative_number(text, signature, positional_idx)
                    {
                        let Some(name) =
                            text.strip_prefix('-').filter(|name| !name.starts_with('-'))
                        else {
                            return Err(Unlowered::Error);
                        };
                        let flags = w::NamedArgument {
                            span: expr.span,
                            name,
                            long: false,
                            value: None,
                        };
                        if self.flag(call, &flags, arguments.get(index), signature)? {
                            index += 1;
                        }
                        continue;
                    }
                    if let Some(positional) = signature.get_positional(positional_idx) {
                        let shape = &positional.shape;
                        let positional_input = match special {
                            Some(SpecialKeyword::Let)
                                if matches!(shape, SyntaxShape::VarWithOptType) =>
                            {
                                output_override = input_type.cloned();
                                input_type
                            }
                            Some(SpecialKeyword::If) if positional_idx >= 1 => input_type,
                            Some(SpecialKeyword::Match) if positional.name == "match_block" => {
                                input_type
                            }
                            _ => None,
                        };
                        let compile_errors = self.working_set.compile_errors.len();
                        let value = if is_multispan_shape(shape) {
                            // The items `calculate_end_span` gives the positional, from this one.
                            let item = index - 1;
                            let end = item
                                + self.multi_item_count(signature, arguments, item, positional_idx);
                            let (value, next) = self.multispan_value(
                                arguments,
                                item,
                                end,
                                shape,
                                positional_input,
                            )?;
                            index = next;
                            value
                        } else {
                            self.value(expr, shape, positional_input)?
                        };
                        // `try`'s handler is a closure, compiled on its own, so a `break` in it
                        // is outside any loop: `parse_internal_call` drops that error when it
                        // is the handler's own (its closure has no IR), for each positional
                        // whose shape lists the `catch` keyword (`try_call` does the same).
                        if let SyntaxShape::OneOf(shapes) = shape
                            && shapes.iter().any(|shape| {
                                matches!(shape, SyntaxShape::Keyword(keyword, _) if keyword == b"catch")
                            })
                            && let [CompileError::NotInALoop { .. }] =
                                &self.working_set.compile_errors[compile_errors..]
                            && let Expr::Keyword(clause) = &value.expr
                            && let Expr::Closure(block_id) = clause.expr.expr
                            && self.working_set.get_block(block_id).ir_block.is_none()
                        {
                            self.working_set.compile_errors.truncate(compile_errors);
                        }
                        match special {
                            Some(SpecialKeyword::Match) if positional.name == "match_block" => {
                                output_override = Some(value.ty.clone());
                            }
                            // `if`'s branches: the block after the condition, the `else` value.
                            Some(SpecialKeyword::If)
                                if positional_idx == 1 && matches!(value.expr, Expr::Block(_)) =>
                            {
                                let ty = self.branch_type(&value, input_type);
                                output_override = Some(match output_override {
                                    Some(existing) => existing.union(ty),
                                    None => ty,
                                });
                            }
                            Some(SpecialKeyword::If)
                                if positional_idx == 2
                                    && let Expr::Keyword(keyword) = &value.expr =>
                            {
                                let ty = self.branch_type(&keyword.expr, input_type);
                                output_override = Some(match output_override {
                                    Some(existing) => existing.union(ty),
                                    None => ty,
                                });
                            }
                            _ => {}
                        }
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
        Ok(output_override)
    }

    /// The value of a positional of the multi-item `shape` that starts at `arguments[index]`,
    /// read from the items before `end` (`calculate_end_span`) as `parse_multispan_value` reads
    /// them. Returns it and the index of the item after the last one it read: a variable
    /// without a type, a keyword and a one-item shape stop early, a row condition, a math
    /// expression, an expression and a signature read up to `end`. A row condition or a math
    /// expression of several items, which the winnow parser read one by one, it reads again as
    /// one (`reread`); an expression or a signature of several is left to the classic parser.
    fn multispan_value(
        &mut self,
        arguments: &[w::Argument<'s>],
        index: usize,
        end: usize,
        shape: &SyntaxShape,
        input_type: Option<&Type>,
    ) -> Lowered<(Expression, usize)> {
        // No item before the next keyword's: the classic parser's missing positional.
        if end <= index {
            return Err(Unlowered::Error);
        }
        let Some(w::Argument::Positional(item)) = arguments.get(index) else {
            return Err(Unlowered::Unsupported("multi-item argument"));
        };
        let text = self.text(item.span);
        let one_item = end == index + 1;
        let value = match shape {
            SyntaxShape::OneOf(shapes) => {
                return self.multispan_one_of(arguments, index, end, shapes, input_type);
            }
            SyntaxShape::Keyword(keyword, value_shape) => {
                return self.keyword_value(arguments, index, end, keyword, value_shape, input_type);
            }
            // Without a `:` after it, the name is the variable's only item.
            SyntaxShape::VarWithOptType if !text.ends_with(':') => {
                return Ok((self.variable_item(item, input_type)?, index + 1));
            }
            // `parse_math_expression` reads `if` or `match` and the items after it as a call
            // (`parse_call`), and either alone as an error.
            SyntaxShape::MathExpression | SyntaxShape::Expression
                if matches!(text, "if" | "match") =>
            {
                if one_item {
                    return Err(Unlowered::Error);
                }
                self.items_call(item, &arguments[index + 1..end], input_type)?
            }
            SyntaxShape::RowCondition if one_item => self.row_condition(item)?,
            SyntaxShape::RowCondition => self.reread(
                items_span(arguments, index, end),
                ItemsValue::RowCondition,
                None,
            )?,
            SyntaxShape::MathExpression if one_item => self.math(item, input_type)?,
            SyntaxShape::MathExpression => self.reread(
                items_span(arguments, index, end),
                ItemsValue::Math,
                input_type,
            )?,
            // `parse_expression` reads one item that looks like a value as a math expression.
            SyntaxShape::Expression
                if one_item && is_math_expression_like(self.working_set, self.span(item.span)) =>
            {
                self.math(item, input_type)?
            }
            // A signature of one item, which `parse_full_signature` reads as the value parser
            // reads the shape (`parse_signature`).
            SyntaxShape::Signature | SyntaxShape::ExternalSignature
                if one_item && text.starts_with('[') =>
            {
                let span = self.span(item.span);
                self.checked(|ws| classic_value(ws, span, shape, None))?
            }
            _ if !is_multispan_shape(shape) => {
                return Ok((self.value(item, shape, input_type)?, index + 1));
            }
            _ => return Err(Unlowered::Unsupported("multi-item argument")),
        };
        Ok((value, end))
    }

    /// The text of `span`, which the winnow parser read otherwise (a call's items one at a time,
    /// an alias's target as a call), read again by it as the one `value` the classic parser
    /// reads there, with the commands the working set knows now, and lowered; a math
    /// expression or an alias's target gets `input_type`. Text it does not read so is a syntax
    /// error there, which the classic parser reports too (`if true {} --help`: the condition
    /// takes `true {}`). The working set answers the parser's questions through a cell while it
    /// parses, as in the driver.
    pub(super) fn reread(
        &mut self,
        span: WSpan,
        value: ItemsValue,
        input_type: Option<&Type>,
    ) -> Lowered<Expression> {
        let cell = RefCell::new(&mut *self.working_set);
        let Some(tree) = parse_items(self.source, span, EngineLookup::new(&cell), value) else {
            return Err(Unlowered::Error);
        };
        let mut working_set = cell.borrow_mut();
        let mut lower = Lower {
            working_set: &mut working_set,
            source: self.source,
            offset: self.offset,
            row_var: self.row_var,
            alias_target: self.alias_target,
        };
        match value {
            ItemsValue::Math => lower.math(&tree, input_type),
            ItemsValue::RowCondition => lower.row_condition(&tree),
            ItemsValue::AliasTarget => lower.expression(&tree, input_type),
        }
    }

    /// A multi-item `oneof` (`parse_oneof` with `multispan`): the first of `shapes` that reads
    /// the items from `index` without an error, in the order [`Self::one_of`] tries them. A
    /// shape whose reading is left to the classic parser ends the search, since the classic
    /// parser might read it without an error.
    fn multispan_one_of(
        &mut self,
        arguments: &[w::Argument<'s>],
        index: usize,
        end: usize,
        shapes: &[SyntaxShape],
        input_type: Option<&Type>,
    ) -> Lowered<(Expression, usize)> {
        let defer_barewords = !matches!(
            self.text(arguments[index].span()).as_bytes().first(),
            Some(b'$' | b'(' | b'{')
        );
        let deferred = |shape: &&SyntaxShape| defer_barewords && accepts_any_bareword(shape);
        let ordered = shapes
            .iter()
            .filter(|shape| !deferred(shape))
            .chain(shapes.iter().filter(|shape| deferred(shape)));
        for shape in ordered {
            let errors = self.working_set.parse_errors.len();
            let warnings = self.working_set.parse_warnings.len();
            let compile_errors = self.working_set.compile_errors.len();
            match self.multispan_value(arguments, index, end, shape, input_type) {
                Ok(value) if self.working_set.parse_errors.len() == errors => return Ok(value),
                Err(unsupported @ Unlowered::Unsupported(_)) => return Err(unsupported),
                _ => {
                    self.working_set.parse_errors.truncate(errors);
                    self.working_set.parse_warnings.truncate(warnings);
                    self.working_set.compile_errors.truncate(compile_errors);
                }
            }
        }
        Err(Unlowered::Error)
    }

    /// A keyword parameter's argument, `keyword value`, from `arguments[index]` (the `Keyword`
    /// case of `parse_multispan_value`): the item at `index` must be the keyword and the value
    /// is read from the items after it, before `end`, with `shape`. Another word, or no item
    /// left for the value, is the classic parser's error.
    fn keyword_value(
        &mut self,
        arguments: &[w::Argument<'s>],
        index: usize,
        end: usize,
        keyword: &[u8],
        shape: &SyntaxShape,
        input_type: Option<&Type>,
    ) -> Lowered<(Expression, usize)> {
        let word = arguments[index].span();
        if self.text(word).as_bytes() != keyword || index + 1 >= end {
            return Err(Unlowered::Error);
        }
        let (value, next) = self.multispan_value(arguments, index + 1, end, shape, input_type)?;
        let span = self.span(word);
        let ty = value.ty.clone();
        let full = span.merge(value.span);
        let keyword = Keyword {
            keyword: keyword.into(),
            span,
            expr: value,
        };
        Ok((self.node(Expr::Keyword(Box::new(keyword)), full, ty), next))
    }

    /// `head` and the `arguments` after it, items of another call, read as one call
    /// (`parse_call`), as `parse_math_expression` reads an `if` or a `match` among its items.
    fn items_call(
        &mut self,
        head: &w::Expression<'s>,
        arguments: &[w::Argument<'s>],
        input_type: Option<&Type>,
    ) -> Lowered<Expression> {
        let span = arguments
            .last()
            .map_or(head.span, |last| head.span.merge(last.span()));
        let call = w::Call {
            head: w::CallHead {
                name: self.text(head.span).into(),
                span: head.span,
            },
            arguments: arguments.to_vec(),
            sigil: None,
            wrapped: false,
        };
        let e = w::Expression::new(w::Expr::Call(call.clone()), span);
        self.call(&e, &call, input_type)
    }

    /// How many of `arguments`, from the one at `index`, the multi-item positional
    /// `positional_idx` of `signature` takes (`calculate_end_span`): the rest of the call for a
    /// signature with a rest parameter; up to a later keyword parameter's keyword, as many as
    /// leave one for each positional between (`first_kw_idx`); else all but one for each
    /// required positional after it.
    fn multi_item_count(
        &self,
        signature: &nu_protocol::Signature,
        arguments: &[w::Argument<'s>],
        index: usize,
        positional_idx: usize,
    ) -> usize {
        if signature.rest_positional.is_some() {
            return arguments.len() - index;
        }
        let keyword = (positional_idx + 1..signature.num_positionals()).find_map(|idx| {
            let Some(SyntaxShape::Keyword(keyword, _)) = signature
                .get_positional(idx)
                .map(|positional| &positional.shape)
            else {
                return None;
            };
            let offset = arguments[index..]
                .iter()
                .position(|argument| self.text(argument.span()).as_bytes() == keyword.as_slice())?;
            Some((idx, index + offset))
        });
        let end = match keyword {
            Some((keyword_idx, keyword_item)) => {
                let between = keyword_idx - positional_idx - 1;
                if between >= keyword_item - index {
                    keyword_item
                } else {
                    keyword_item - between
                }
            }
            None => {
                let remaining_items = arguments.len() - (index + 1);
                let remaining_positionals = signature
                    .required_positional
                    .len()
                    .saturating_sub(positional_idx + 1);
                index + 1 + remaining_items.saturating_sub(remaining_positionals)
            }
        };
        end - index
    }

    /// A variable written as one item, `x` (`parse_var_with_opt_type`): created in scope, not
    /// mutable, with the type `input_type` (`any` without one); its declaration is typed `any`.
    /// A name that is no variable's is the classic parser's error. Unlike the bindings'
    /// variables (`variable_declaration`), it is in scope at once, as the classic parser puts it
    /// there: a call that reads one is a call for help, which the winnow parser has already
    /// checked, so no later error hands it back.
    fn variable_item(
        &mut self,
        e: &w::Expression<'s>,
        input_type: Option<&Type>,
    ) -> Lowered<Expression> {
        let span = self.span(e.span);
        let name = self.text(e.span).as_bytes();
        if name.iter().any(|byte| b" \"'`".contains(byte)) || !is_variable(name) {
            return Err(Unlowered::Error);
        }
        self.checked(|ws| ensure_not_reserved_variable_name(ws, name, span))?;
        let ty = input_type.cloned().unwrap_or(Type::Any);
        let var_id = self
            .working_set
            .add_variable(name.to_vec(), span, ty, false);
        Ok(self.node(Expr::VarDecl(var_id), span, Type::Any))
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
    pub(super) fn flag_value(
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
    /// (`extern`, `def --wrapped`), an error otherwise. An `extern --strict` command takes only
    /// the flags `strict_extern_forwards` lets through (`parse_internal_call`).
    fn unknown_flag(
        &mut self,
        call: &mut Call,
        token: Span,
        signature: &nu_protocol::Signature,
    ) -> Lowered<()> {
        if !signature.allows_unknown_args
            || signature.rejects_unknown_flags
                && !strict_extern_forwards(self.working_set.get_span_contents(token))
        {
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

    /// A call whose command the classic parser does not find, which `parse_call` makes an
    /// external call (`parse_external_call`): in an alias's body, a command the block defines
    /// later (`alias s = shout; def shout [] {}`). The head's first word is the external command,
    /// its other words and the arguments are the arguments. A shorter head that names a command
    /// would be a call to it, which the winnow tree does not have.
    fn call_as_external(
        &mut self,
        e: &w::Expression<'s>,
        call: &w::Call<'s>,
    ) -> Lowered<Expression> {
        // The head's words, which may be apart by more than one blank (`cargo   make`).
        let base = call.head.span.start;
        let bytes = self.text(call.head.span).as_bytes();
        let mut words = Vec::new();
        let mut index = 0;
        while index < bytes.len() {
            if bytes[index].is_ascii_whitespace() {
                index += 1;
                continue;
            }
            let start = index;
            while index < bytes.len() && !bytes[index].is_ascii_whitespace() {
                index += 1;
            }
            words.push(WSpan::new(base + start, base + index));
        }
        let shorter_head_is_a_command = (1..words.len()).any(|len| {
            let name: Vec<&str> = words[..len].iter().map(|word| self.text(*word)).collect();
            self.working_set
                .find_decl(name.join(" ").as_bytes())
                .is_some()
        });
        let Some((first, rest)) = words.split_first() else {
            return Err(Unlowered::Unsupported("unresolved command"));
        };
        if shorter_head_is_a_command {
            return Err(Unlowered::Unsupported("unresolved command"));
        }
        let head_span = self.span(*first);
        let head = self.checked(|ws| parse_external_string(ws, head_span))?;
        let mut arguments = Vec::with_capacity(rest.len() + call.arguments.len());
        for word in rest {
            let span = self.span(*word);
            arguments.push(ExternalArgument::Regular(
                self.checked(|ws| parse_regular_external_arg(ws, span))?,
            ));
        }
        for argument in &call.arguments {
            arguments.push(self.call_argument_as_external(argument)?);
        }
        let span = self.span(e.span);
        Ok(self.node(
            Expr::ExternalCall(Box::new(head), arguments.into()),
            span,
            Type::Any,
        ))
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

/// The span of the items `arguments[index..end]`.
fn items_span(arguments: &[w::Argument<'_>], index: usize, end: usize) -> WSpan {
    arguments[index].span().merge(arguments[end - 1].span())
}

/// The keywords `parse_internal_call` reads some positionals of in a way of its own
/// (`SpecialCmd`; `def`'s body is `def_body_span`'s).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum SpecialKeyword {
    Let,
    Match,
    If,
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
