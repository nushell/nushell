//! Keyword statements: `if`, `match`, `while`, `loop`, `try`, `return`, `break`, `continue`
//! (ordinary calls to keyword commands in the classic parser), and `let`, `mut`, `const`, `for`,
//! `def`, `extern`, `alias`, `type` and `export-env` (the classic parser's `parse_let`,
//! `parse_mut`, `parse_const`, `parse_for`, `parse_def`, `parse_extern`, `parse_alias`,
//! `parse_type_alias` and `parse_export_env`), with the attributes and the `export` around a
//! definition.

use std::{collections::HashMap, sync::Arc};

use nu_protocol::eval_const::eval_constant;
use nu_protocol::{
    CompileError, DeclId, ParseError, PositionalArg, ShellError, Span, Spanned, SyntaxShape, Type,
    TypeSet, Value,
    ast::{
        Argument, Attribute, AttributeBlock, Call, Expr, Expression, Keyword, MatchPattern,
        Pattern, Pipeline,
    },
};

use super::{BlockOf, Lower, Lowered, Unlowered, is_missing_argument};
use crate::winnow::syntax::{ItemsValue, Span as WSpan, ast as w};
use crate::{
    exportable::Exportable,
    lex::{is_assignment_operator, lex, lex_signature},
    parse_alias::{
        alias_name, alias_name_fits_module, check_alias_name, finish_alias, report_incomplete_alias,
    },
    parse_calls::CallKind,
    parse_captures_compile::compile_block_with_id,
    parse_def::{
        DefCall, detect_params_in_name, finish_def, finish_extern, finish_for, yielded_type,
    },
    parse_expressions::{
        is_env_variable_name, is_math_expression_like, parse_expression, parse_value,
    },
    parse_helpers::{garbage, is_variable},
    parse_keywords::find_keyword_decl,
    parse_module::warp_export_call,
    parse_patterns::parse_pattern,
    parse_shape_specs::parse_type,
    parse_signatures::ensure_not_reserved_variable_name,
    parse_type_alias::{finish_type_alias, type_alias_layout_error},
    type_check::{check_pipeline_type, type_compatible},
};

impl<'s> Lower<'_, '_, 's> {
    /// The call to keyword `name` that `e` is, its head the keyword. The head's span is
    /// registered in the working set, as `parse_internal_call` registers it (as do the other
    /// calls the lowering builds).
    fn keyword_call(&mut self, e: &w::Expression<'s>, name: &str) -> Lowered<(Call, DeclId)> {
        // The statement of a call through an alias of the keyword: `if`, `match` and `try`
        // take it (`keyword_or_alias_call`), the winnow parser builds no other.
        if self.text(e.span).split_ascii_whitespace().next() != Some(name) {
            return Err(Unlowered::Unsupported("call through an alias of a keyword"));
        }
        let decl_id = self
            .working_set
            .find_decl(name.as_bytes())
            .ok_or(Unlowered::Error)?;
        let head = self.keyword_span(e, name);
        let mut call = Call::new(head);
        call.decl_id = decl_id;
        let _ = self.working_set.add_span(head);
        Ok((call, decl_id))
    }

    /// The call of the statement of keyword `name` headed by `keyword`: the keyword's
    /// ([`Self::keyword_call`]), or, when `keyword` names an alias of it (`alias m = match`,
    /// then `m 1 {..}`), the alias's as `parse_internal_call` makes it: the keyword's call the
    /// alias holds, with the alias's name as its head. Returns the call, the command whose
    /// signature checks and types it (the alias), and whether it is an alias's: the classic
    /// parser reads an alias's call without the keyword's special readings (`SpecialCmd` wants
    /// a keyword), so its blocks get no input and its output is the signature's.
    fn keyword_or_alias_call(
        &mut self,
        e: &w::Expression<'s>,
        keyword: WSpan,
        name: &str,
    ) -> Lowered<(Call, DeclId, bool)> {
        let alias_name = self.text(keyword);
        if alias_name == name {
            let (call, decl_id) = self.keyword_call(e, name)?;
            return Ok((call, decl_id, false));
        }
        let unexpected = || Unlowered::Unsupported("call through an alias of a keyword");
        let decl_id = self
            .working_set
            .find_decl(alias_name.as_bytes())
            .ok_or_else(unexpected)?;
        let Some(Expr::Call(wrapped)) = self
            .working_set
            .get_decl(decl_id)
            .as_alias()
            .map(|alias| &alias.wrapped_call.expr)
        else {
            return Err(unexpected());
        };
        let mut call = (**wrapped).clone();
        call.head = self.span(keyword);
        let _ = self.working_set.add_span(call.head);
        Ok((call, decl_id, true))
    }

    /// The end of a keyword's call (`parse_internal_call`): the missing-argument check, and
    /// the output type the keyword's signature gives for `input_type`. Only `CallKind::Invalid`
    /// is handed back: `CallKind::Help` needs a `--help` flag, which no keyword call the
    /// lowering builds has.
    fn finish_keyword_call(
        &mut self,
        call: &Call,
        decl_id: DeclId,
        input_type: Option<&Type>,
    ) -> Lowered<Type> {
        let signature = self.working_set.get_signature_shared(decl_id);
        self.check_call(call.head, &signature, call)?;
        Ok(self
            .working_set
            .call_output_type(decl_id, &signature, input_type)
            .unwrap_or(Type::Error))
    }

    /// A block argument of a keyword, `{ ... }` (`parse_block_expression`), or the `value` in
    /// its place (`if $c $env.f`, `try {a: 1}.a`): a variable, a subexpression or a brace value
    /// read as the value parser reads it for the `block` shape, and type-checked as a block
    /// (`parse_internal_call`). Such a call is the IR compiler's error.
    fn block_argument(
        &mut self,
        block: &w::Block<'s>,
        value: Option<&w::Expression<'s>>,
        input_type: Option<&Type>,
    ) -> Lowered<Expression> {
        if let Some(value) = value {
            // `parse_internal_call`'s check of the positional: a value of another type (a
            // record, `while true {a: 1}`) is reported, garbage.
            let value = self.value(value, &SyntaxShape::Block, input_type)?;
            return Ok(self.positional_value(value, &SyntaxShape::Block));
        }
        let span = self.braces_span(block)?;
        self.block_expression(span, block, input_type)
    }

    /// The span of a block's braces, from the span of its inside: a winnow block covers the text
    /// between its braces, while the classic parser's block and closure expressions cover the
    /// braces. A block that is not between braces in the source is left to the classic parser.
    pub(super) fn braces_span(&self, block: &w::Block<'s>) -> Lowered<WSpan> {
        let (start, end) = (block.span.start, block.span.end);
        if start == 0
            || !self.source[..start].ends_with('{')
            || !self.source[end..].starts_with('}')
        {
            return Err(Unlowered::Unsupported("block without braces"));
        }
        Ok(WSpan::new(start - 1, end + 1))
    }

    /// [`Self::braces_span`] of a body written with parameters (`{|x| }`), whose block starts
    /// after them: from the `{` before the parameters to the `}` after the block.
    fn braces_span_with_params(
        &self,
        params: &w::Signature<'s>,
        block: &w::Block<'s>,
    ) -> Lowered<WSpan> {
        let before = self.source[..params.span.start].trim_end();
        if !before.ends_with('{') || !self.source[block.span.end..].starts_with('}') {
            return Err(Unlowered::Unsupported("block without braces"));
        }
        Ok(WSpan::new(before.len() - 1, block.span.end + 1))
    }

    /// `if condition { then } else { otherwise }`: a call to `if`, whose output is the union of
    /// its branches' outputs (with `nothing` when there is no `else`), as `parse_internal_call`
    /// overrides the output of `if`: a value in the `then` block's place gives none, so with no
    /// `else` the output is the signature's. The condition gets no input, the branches
    /// `input_type`. The `else` branch, a block or an expression (`else if ...`), is a `Keyword`
    /// argument. Through an alias of `if`, the branches get no input and the output is the
    /// signature's (`keyword_or_alias_call`).
    pub(super) fn if_call(
        &mut self,
        e: &w::Expression<'s>,
        if_expr: &w::If<'s>,
        input_type: Option<&Type>,
    ) -> Lowered<Expression> {
        let (mut call, decl_id, aliased) = self.keyword_or_alias_call(e, if_expr.keyword, "if")?;
        let branch_input = if aliased { None } else { input_type };
        let condition = self.math(&if_expr.condition, None)?;
        call.add_positional(condition);
        let then = self.block_argument(
            &if_expr.then_block,
            if_expr.then_value.as_deref(),
            branch_input,
        )?;
        let mut output = match &then.expr {
            Expr::Block(_) if !aliased => Some(self.branch_type(&then, input_type)),
            _ => None,
        };
        call.add_positional(then);
        if let Some(branch) = &if_expr.else_branch {
            let body = match &branch.body.expr {
                w::Expr::Block(block) => {
                    self.block_expression(branch.body.span, block, branch_input)?
                }
                _ => self.expression(&branch.body, branch_input)?,
            };
            let branch_type = self.branch_type(&body, input_type);
            output = Some(match output {
                Some(output) => output.union(branch_type),
                None => branch_type,
            });
            let keyword_span = self.span(branch.keyword);
            let span = keyword_span.merge(body.span);
            let ty = body.ty.clone();
            let keyword = Keyword {
                keyword: b"else".as_slice().into(),
                span: keyword_span,
                expr: body,
            };
            let argument = self.node(Expr::Keyword(Box::new(keyword)), span, ty);
            call.add_positional(argument);
        } else {
            output = output.map(|output| output.union(Type::Nothing));
        }
        // The branches' union, else the signature's output.
        let signature_output = self.finish_keyword_call(&call, decl_id, input_type)?;
        let span = self.span(e.span);
        let output = match aliased {
            true => signature_output,
            false => output.unwrap_or(signature_output),
        };
        Ok(self.node(Expr::Call(Box::new(call)), span, output))
    }

    /// The type a branch of `if` or `match` gives: its block's output, or the input for an
    /// empty block (as `parse_internal_call` and `parse_match_block_expression` compute it).
    pub(super) fn branch_type(&self, branch: &Expression, input_type: Option<&Type>) -> Type {
        match &branch.expr {
            Expr::Block(block_id) => {
                let block = self.working_set.get_block(*block_id);
                if block.pipelines.is_empty() {
                    input_type.cloned().unwrap_or(Type::Any)
                } else {
                    block.output_type()
                }
            }
            _ => branch.ty.clone(),
        }
    }

    /// `while condition { body }`: neither the condition nor the body gets the pipeline's input.
    pub(super) fn while_call(
        &mut self,
        e: &w::Expression<'s>,
        while_loop: &w::While<'s>,
    ) -> Lowered<Expression> {
        let (mut call, decl_id) = self.keyword_call(e, "while")?;
        let condition = self.math(&while_loop.condition, None)?;
        call.add_positional(condition);
        let body = self.block_argument(&while_loop.body, while_loop.body_value.as_deref(), None)?;
        call.add_positional(body);
        let output = self.finish_keyword_call(&call, decl_id, None)?;
        let span = self.span(e.span);
        Ok(self.node(Expr::Call(Box::new(call)), span, output))
    }

    /// `loop { body }`.
    pub(super) fn loop_call(
        &mut self,
        e: &w::Expression<'s>,
        body: &w::Loop<'s>,
    ) -> Lowered<Expression> {
        let (mut call, decl_id) = self.keyword_call(e, "loop")?;
        let block = self.block_argument(&body.body, body.body_value.as_deref(), None)?;
        call.add_positional(block);
        let output = self.finish_keyword_call(&call, decl_id, None)?;
        let span = self.span(e.span);
        Ok(self.node(Expr::Call(Box::new(call)), span, output))
    }

    /// `try { body } catch {|err| ... } finally { ... }`: a call to `try` with the body as a
    /// block and each handler as a `Keyword` argument holding a closure.
    ///
    /// A handler's closure is compiled on its own as it is lowered, so a `break` or `continue`
    /// in it reports `NotInALoop` even when the `try` is in a loop. `parse_internal_call` drops
    /// that error for each of `try`'s handler parameters (both list `catch` in their shapes, so
    /// a `finally` handler too) when it is the handler's own: the only compile error the
    /// handler added, and its closure has no IR. An error from a closure or `def` body nested
    /// in the handler, whose closure then did compile, stands. Hence the compile errors are
    /// counted before each handler. A value in a handler's place (`catch $handler`) is read
    /// with the handler's shape in `try`'s signature, as the value of its keyword, and must be
    /// typed as a closure. The classic parser reads `try` with no special readings, so a call
    /// through an alias of it differs only in its head and signature (`keyword_or_alias_call`).
    pub(super) fn try_call(
        &mut self,
        e: &w::Expression<'s>,
        try_expr: &w::Try<'s>,
        input_type: Option<&Type>,
    ) -> Lowered<Expression> {
        let (mut call, decl_id, _) = self.keyword_or_alias_call(e, try_expr.keyword, "try")?;
        let body = self.block_argument(&try_expr.body, try_expr.body_value.as_deref(), None)?;
        call.add_positional(body);
        for handler in &try_expr.handlers {
            let keyword = match handler.kind {
                w::HandlerKind::Catch => "catch",
                w::HandlerKind::Finally => "finally",
            };
            let compile_errors = self.working_set.compile_errors.len();
            let (closure, value_shape) = match &handler.body.expr {
                w::Expr::Closure(closure) => (
                    self.closure(
                        handler.body.span,
                        closure.params.as_ref(),
                        &closure.body,
                        &SyntaxShape::Closure(None),
                        None,
                    )?,
                    None,
                ),
                _ => {
                    let signature = self.working_set.get_signature_shared(decl_id);
                    let shape = handler_shape(&signature, keyword)
                        .ok_or(Unlowered::Unsupported("handler that is not a closure"))?;
                    (self.value(&handler.body, &shape, None)?, Some(shape))
                }
            };
            if let [CompileError::NotInALoop { .. }] =
                &self.working_set.compile_errors[compile_errors..]
                && let Expr::Closure(block_id) = closure.expr
                && self.working_set.get_block(block_id).ir_block.is_none()
            {
                self.working_set.compile_errors.truncate(compile_errors);
            }
            let keyword_span = self.span(handler.keyword);
            let span = keyword_span.merge(closure.span);
            let ty = closure.ty.clone();
            let keyword = Keyword {
                keyword: keyword.as_bytes().into(),
                span: keyword_span,
                expr: closure,
            };
            let argument = self.node(Expr::Keyword(Box::new(keyword)), span, ty);
            // `parse_internal_call`'s check of the positional, the keyword and its value typed as
            // the handler: one of another type (a record, `catch {a: 1}`) is reported, garbage.
            let argument = match value_shape {
                Some(shape) => self.positional_value(argument, &shape),
                None => argument,
            };
            call.add_positional(argument);
        }
        let output = self.finish_keyword_call(&call, decl_id, input_type)?;
        let span = self.span(e.span);
        Ok(self.node(Expr::Call(Box::new(call)), span, output))
    }

    /// `match value { pattern => result, ... }`: a call to `match` with the value and a
    /// `MatchBlock` of the arms (`parse_match_block_expression`). Each arm has a scope of its
    /// own, in which its pattern declares its variables before its guard and its result are
    /// lowered. The value gets no input, the results `input_type`. The output is the union of
    /// the arms' outputs (with `nothing` when no arm matches everything). A value in the arms'
    /// place (a closure, a record, `$x`) is read as the value parser reads it for the
    /// `match_block` shape, and the call takes its type (`parse_internal_call`); such a call is
    /// the IR compiler's error. Through an alias of `match`, the arms get no input and the
    /// output is the signature's (`keyword_or_alias_call`).
    pub(super) fn match_call(
        &mut self,
        e: &w::Expression<'s>,
        match_expr: &w::Match<'s>,
        input_type: Option<&Type>,
    ) -> Lowered<Expression> {
        let (mut call, decl_id, aliased) =
            self.keyword_or_alias_call(e, match_expr.keyword, "match")?;
        let arms_input = if aliased { None } else { input_type };
        let value = self.value(&match_expr.value, &SyntaxShape::Any, None)?;
        call.add_positional(value);
        let block = if let Some(value_block) = &match_expr.value_block {
            let block = self.reported_value(value_block, &SyntaxShape::MatchBlock, arms_input)?;
            self.positional_value(block, &SyntaxShape::MatchBlock)
        } else {
            let mut arms = Vec::with_capacity(match_expr.arms.len());
            let mut output = Type::one_of([]);
            for arm in &match_expr.arms {
                let (pattern, result) = self.in_scope(|this| {
                    let mut pattern = this.pattern(&arm.pattern)?;
                    if let Some(guard) = &arm.guard {
                        pattern.guard = Some(Box::new(this.math(guard, None)?));
                    }
                    let result = this.match_result(&arm.body, arms_input)?;
                    Ok((pattern, result))
                })?;
                output = output.union(self.branch_type(&result, arms_input));
                arms.push((pattern, result));
            }
            if !arms.iter().any(|(pattern, _)| pattern.is_wildcard()) {
                output = output.union(Type::Nothing);
            }
            let block_span = self.span(match_expr.block_span);
            self.node(Expr::MatchBlock(arms), block_span, output)
        };
        // The call of `match` takes the type of its arms, not the signature's.
        let arms_output = block.ty.clone();
        call.add_positional(block);
        let signature_output = self.finish_keyword_call(&call, decl_id, input_type)?;
        let output = if aliased {
            signature_output
        } else {
            arms_output
        };
        let span = self.span(e.span);
        Ok(self.node(Expr::Call(Box::new(call)), span, output))
    }

    /// A match arm's pattern, read by the classic pattern parser (`parse_pattern`), which also
    /// declares its variables and reports its errors (a reserved variable name, a malformed
    /// literal), as there. The alternatives of an or-pattern (`a | b`) are read one by one,
    /// as `parse_match_block_expression` reads them.
    fn pattern(&mut self, pattern: &w::MatchPattern<'s>) -> Lowered<MatchPattern> {
        match &pattern.pattern {
            w::Pattern::Or(alternatives) => {
                let mut out = Vec::with_capacity(alternatives.len());
                for alternative in alternatives {
                    out.push(self.pattern(alternative)?);
                }
                // From the first alternative to the last, which is what the winnow span covers.
                let span = self.span(pattern.span);
                Ok(MatchPattern {
                    pattern: Pattern::Or(out),
                    guard: None,
                    span,
                })
            }
            _ => {
                let span = self.span(pattern.span);
                Ok(parse_pattern(self.working_set, span))
            }
        }
    }

    /// The result of a match arm, which the classic parser reads as `oneof<block, expression>`:
    /// braces without parameters are a block (`{}` included, while `{a: 1}` stays a record, as
    /// `parse_brace_expr` decides for a block shape); anything else, a closure with parameters
    /// included, is an expression.
    fn match_result(
        &mut self,
        body: &w::Expression<'s>,
        input_type: Option<&Type>,
    ) -> Lowered<Expression> {
        match &body.expr {
            w::Expr::Block(block) => self.block_expression(body.span, block, input_type),
            w::Expr::Closure(closure) if closure.params.is_none() => {
                self.block_expression(body.span, &closure.body, input_type)
            }
            w::Expr::Record(_) if self.text(body.span).starts_with('{') => {
                self.value(body, &SyntaxShape::Block, input_type)
            }
            _ => self.expression(body, input_type),
        }
    }

    /// `return` with an optional value.
    pub(super) fn return_call(
        &mut self,
        e: &w::Expression<'s>,
        value: &w::Return<'s>,
    ) -> Lowered<Expression> {
        let (mut call, decl_id) = self.keyword_call(e, "return")?;
        if let Some(value) = &value.value {
            let value = self.value(value, &SyntaxShape::Any, None)?;
            call.add_positional(value);
        }
        let output = self.finish_keyword_call(&call, decl_id, None)?;
        let span = self.span(e.span);
        Ok(self.node(Expr::Call(Box::new(call)), span, output))
    }

    /// `break` and `continue`.
    pub(super) fn bare_keyword_call(
        &mut self,
        e: &w::Expression<'s>,
        name: &str,
    ) -> Lowered<Expression> {
        let (call, decl_id) = self.keyword_call(e, name)?;
        let output = self.finish_keyword_call(&call, decl_id, None)?;
        let span = self.span(e.span);
        Ok(self.node(Expr::Call(Box::new(call)), span, output))
    }

    /// The variable of `let`, `mut`, `const` or `for` (`parse_var_with_opt_type`), created
    /// with its written type, or else with `input_type` (`any` without one), which the caller
    /// may replace. Returns its `VarDecl` expression, the written type and the variable's name.
    ///
    /// The variable is not in scope yet. The caller puts it there (`insert_variable_into_scope`)
    /// where the classic parser declares it or, when a later check can still hand the statement
    /// back, once that check passed: the classic parser, parsing the statement again, must not
    /// find the variable (`let x: string = $x` reads the `x` before it).
    pub(super) fn variable_declaration(
        &mut self,
        name: &crate::winnow::syntax::Spanned<&'s str>,
        ty: Option<&w::TypeAnnotation<'s>>,
        mutable: bool,
        input_type: Option<&Type>,
    ) -> Lowered<(Expression, Option<Type>, Vec<u8>)> {
        // The variable's item keeps a `$` written before the name (`for $x in ...`).
        let name_span = self.span(name.span);
        let (name_span, var_name) = if self.source[..name.span.start].ends_with('$') {
            (
                Span::new(name_span.start - 1, name_span.end),
                format!("${}", name.item).into_bytes(),
            )
        } else {
            (name_span, name.item.as_bytes().to_vec())
        };
        // A name with a space or a quote, or one that is no variable name, which the winnow
        // parser keeps for a binding (`let 'a b' = 1`, `let $x-y = 1`): nu reports it and goes on
        // with garbage for the variable.
        if var_name.iter().any(|byte| b" \"'`".contains(byte)) {
            self.working_set
                .error(ParseError::VariableNotValid(name_span));
            return Ok((garbage(self.working_set, name_span), None, var_name));
        }
        if !is_variable(&var_name) {
            self.working_set
                .error(ParseError::Expected("valid variable name", name_span));
            return Ok((garbage(self.working_set, name_span), None, var_name));
        }
        ensure_not_reserved_variable_name(self.working_set, &var_name, name_span);
        match ty {
            Some(annotation) => {
                let span = self.span(annotation.span);
                let text = self.text(annotation.span).as_bytes();
                // nu reports an unknown type and goes on with it (`parse_var_with_opt_type`),
                // at the first item of the type lexed as a signature (`int=` of `int= 3`).
                let (tokens, _) = lex_signature(text, span.start, &[], b",", true);
                let at = tokens.first().map_or(span, |token| token.span);
                let ty = parse_type(self.working_set, text, at);
                // The classic parser gives the variable the span of the next-to-last item of
                // `name: type...`: the name with its `:` when the type is one item.
                let region = Span::new(name_span.start, span.end);
                let (items, _) = lex(
                    self.working_set.get_span_contents(region),
                    region.start,
                    &[],
                    &[],
                    true,
                );
                let var_span = match items.as_slice() {
                    [.., second_to_last, _] => second_to_last.span,
                    _ => Span::new(name_span.start, name_span.end + 1),
                };
                let var_id =
                    self.working_set
                        .add_variable_without_scope(var_span, ty.clone(), mutable);
                let decl = self.node(Expr::VarDecl(var_id), name_span, ty.clone());
                Ok((decl, Some(ty), var_name))
            }
            // `let a: = 1`: a typed name without its type, which nu declares `any` over the
            // name's item, its colon included, and reports (MissingType).
            None if self.source[name.span.end..].starts_with(':') => {
                let span = Span::new(name_span.start, name_span.end + 1);
                let var_id = self
                    .working_set
                    .add_variable_without_scope(span, Type::Any, mutable);
                self.working_set.error(ParseError::MissingType(span));
                let decl = self.node(Expr::VarDecl(var_id), span, Type::Any);
                Ok((decl, None, var_name))
            }
            None => {
                let var_id = self.working_set.add_variable_without_scope(
                    name_span,
                    input_type.cloned().unwrap_or(Type::Any),
                    mutable,
                );
                let decl = self.node(Expr::VarDecl(var_id), name_span, Type::Any);
                Ok((decl, None, var_name))
            }
        }
    }

    /// The variable of a binding nu reads (`let x = value`): [`Self::variable_declaration`],
    /// then the first item an untyped variable leaves before the `=`, which nu reports
    /// (ExtraTokens) and goes on past (`let a int = 1`, `let a : int = 1`).
    fn binding_variable(
        &mut self,
        binding: &w::Binding<'s>,
        mutable: bool,
        input_type: Option<&Type>,
    ) -> Lowered<(Expression, Option<Type>, Vec<u8>)> {
        let declared =
            self.variable_declaration(&binding.name, binding.ty.as_ref(), mutable, input_type)?;
        if binding.ty.is_none()
            && let Some(eq) = binding.eq
            && !self.source[binding.name.span.end..].starts_with(':')
            && let Some(first) = self
                .items(WSpan::new(binding.name.span.end, eq.start))
                .first()
        {
            self.working_set.error(ParseError::ExtraTokens(*first));
        }
        Ok(declared)
    }

    /// `let name = value` (`parse_let`): the value is a pipeline parsed before the variable is
    /// declared; an untyped variable takes the value's type. The value receives `input_type`
    /// (`ls | let files = $in`).
    pub(super) fn let_statement(
        &mut self,
        element: &w::PipelineElement<'s>,
        binding: &w::Binding<'s>,
        input_type: Option<&Type>,
    ) -> Lowered<Pipeline> {
        self.binding(element, binding, "let", false, input_type)
    }

    /// `mut name = value` (`parse_mut`): as `let`, without the pipeline's input.
    pub(super) fn mut_statement(
        &mut self,
        element: &w::PipelineElement<'s>,
        binding: &w::Binding<'s>,
    ) -> Lowered<Pipeline> {
        self.binding(element, binding, "mut", true, None)
    }

    /// `let` or `mut` (`parse_let`, `parse_mut`), `keyword` being which, in the classic order:
    ///
    /// 1. The value, a pipeline lowered as an `Expr::Block` before the variable exists, so
    ///    that `let x = $x + 1` reads an outer `x`. Its type is what `check_pipeline_type`
    ///    gives for a `let` of one pipeline with an input, else the block's output.
    /// 2. The variable, created with its written type (an untyped `let` variable with
    ///    `input_type` until step 3).
    /// 3. A written type must accept the value's type, or that is reported; then the variable
    ///    is put in scope, and an untyped one takes the value's type.
    ///
    /// A value whose pipeline doesn't take the input type has its errors reported too.
    ///
    /// `let x` or `mut x` without a value is a plain call to the keyword
    /// (`binding_without_value`), as `parse_let` and `parse_mut` read a statement without an
    /// `=`.
    fn binding(
        &mut self,
        element: &w::PipelineElement<'s>,
        binding: &w::Binding<'s>,
        keyword: &str,
        mutable: bool,
        input_type: Option<&Type>,
    ) -> Lowered<Pipeline> {
        let Some(value) = &binding.value else {
            let call = self.binding_without_value(&element.expr, binding, keyword, input_type)?;
            return Ok(Pipeline::from_vec(vec![call]));
        };
        let decl_id = self
            .working_set
            .find_decl(keyword.as_bytes())
            .ok_or(Unlowered::Error)?;
        let rvalue_span = self.span(value.span);
        let block = self.block(value, rvalue_span, false, BlockOf::Value, input_type)?;
        // `mut` gives no input (`mut_statement`), so only a `let` gets here with one.
        let output_type = match (block.pipelines.as_slice(), input_type) {
            ([pipeline], Some(input)) => {
                match check_pipeline_type(self.working_set, pipeline, input) {
                    Ok(output_type) => output_type,
                    Err((output_type, errors)) => {
                        self.working_set.parse_errors.extend(errors);
                        output_type
                    }
                }
            }
            _ => block.output_type(),
        };
        let block_id = self.working_set.add_block(Arc::new(block));
        let rvalue = self.node(Expr::Block(block_id), rvalue_span, output_type);
        let (lvalue, explicit_type, var_name) =
            self.binding_variable(binding, mutable, input_type)?;
        let rhs_type = rvalue.ty.clone();
        if let Some(explicit_type) = &explicit_type
            && !type_compatible(explicit_type, &rhs_type)
        {
            self.working_set.error(ParseError::TypeMismatch(
                explicit_type.clone(),
                rhs_type.clone(),
                rvalue_span,
            ));
        }
        if let Some(var_id) = lvalue.as_var() {
            // In scope once the statement can no longer be handed back (`variable_declaration`).
            self.working_set
                .insert_variable_into_scope(var_name, var_id);
            if explicit_type.is_none() {
                self.working_set.set_variable_type(var_id, rhs_type);
            }
        }
        let head = self.keyword_span(&element.expr, keyword);
        let call = Box::new(Call {
            decl_id,
            head,
            arguments: vec![Argument::Positional(lvalue), Argument::Positional(rvalue)],
            parser_info: HashMap::new(),
        });
        let span = self.span(element.expr.span);
        Ok(Pipeline::from_vec(vec![self.node(
            Expr::Call(call),
            span,
            Type::Any,
        )]))
    }

    /// `for variable in iterable { body }` (`parse_for`): the variable, in the loop's scope,
    /// gets the type of what the iterable yields, and the body's block takes it. In order:
    ///
    /// 1. In the loop's scope, as `parse_internal_call` reads `for`'s arguments: the variable
    ///    is declared (with its written type or `any`), then the iterable and the body (a block,
    ///    compiled with the block the `for` is in) are lowered.
    /// 2. Once the scope is left and the call checked, the body's block gets `for`'s signature
    ///    with the variable as its first parameter, and the variable the type the iterable
    ///    yields (`yielded_type`). The call's output is `nothing`.
    pub(super) fn for_statement(
        &mut self,
        element: &w::PipelineElement<'s>,
        for_loop: &w::For<'s>,
    ) -> Lowered<Expression> {
        let e = &element.expr;
        let (mut call, decl_id) = self.keyword_call(e, "for")?;
        let (var_decl, iterable, body) = self.in_scope(|this| {
            let (var_decl, _, var_name) =
                this.variable_declaration(&for_loop.var, for_loop.ty.as_ref(), false, None)?;
            // In scope at once: `parse_internal_call` declares it before reading the iterable.
            if let Some(var_id) = var_decl.as_var() {
                this.working_set
                    .insert_variable_into_scope(var_name, var_id);
            }
            let iterable = this.value(&for_loop.iterable, &SyntaxShape::Any, None)?;
            let body = this.block_argument(&for_loop.body, for_loop.body_value.as_deref(), None)?;
            Ok((var_decl, iterable, body))
        })?;
        let var_id = var_decl.as_var();
        call.add_positional(var_decl);
        let in_span = self.span(for_loop.in_keyword);
        let keyword_span = in_span.merge(iterable.span);
        let iterable_ty = iterable.ty.clone();
        let keyword = Keyword {
            keyword: b"in".as_slice().into(),
            span: in_span,
            expr: iterable,
        };
        let keyword = self.node(
            Expr::Keyword(Box::new(keyword)),
            keyword_span,
            iterable_ty.clone(),
        );
        call.add_positional(keyword);
        let block_id = body.as_block();
        call.add_positional(body);
        self.finish_keyword_call(&call, decl_id, None)?;

        let signature = self.working_set.get_decl(decl_id).signature();
        if let Some(block_id) = block_id {
            *self.working_set.get_block_mut(block_id).signature = signature;
        }
        let var_type = yielded_type(iterable_ty);
        if let (Some(var_id), Some(block_id)) = (var_id, block_id) {
            self.working_set.set_variable_type(var_id, var_type.clone());
            let block = self.working_set.get_block_mut(block_id);
            block.signature.required_positional.insert(
                0,
                PositionalArg {
                    name: String::new(),
                    desc: String::new(),
                    shape: var_type.to_shape(),
                    var_id: Some(var_id),
                    default_value: None,
                    completion: None,
                },
            );
        }
        let span = self.span(e.span);
        Ok(self.node(Expr::Call(Box::new(call)), span, Type::Nothing))
    }

    /// `def name [signature] { body }` (`parse_def`), with its `attributes`; `command` is the
    /// `def` expression, or the `export` expression around it (for an `export def` in a
    /// module named `module_name`, or in a script). The definition is a call to `def` whose
    /// predeclared command becomes the defined one. Returns the definition's expression and the
    /// command's name and declaration. In the order of `parse_def`:
    ///
    /// 1. The description from the comments, and the attributes with their constant values.
    /// 2. In a scope of its own, the call to `def`: the name; the signature, which declares the
    ///    parameters' variables in that scope; then the body, a closure whose input is the
    ///    signature's input type, marked by `def_body_span` so that `closure` does not compile
    ///    it. Parameters written on the body (`{|x| }`) are the closure's, in its own scope, as
    ///    `parse_closure_expression` reads them; `finish_def` replaces them with the signature.
    /// 3. Once the scope is left, `finish_def` compiles the body and makes the predeclared
    ///    command the defined one.
    /// 4. The attributes and the `export` around the call (`definition_expression`).
    pub(super) fn def(
        &mut self,
        pipeline: &w::Pipeline<'s>,
        command: &w::Expression<'s>,
        def: &w::Def<'s>,
        attributes: &[w::Attribute<'s>],
        module_name: Option<&[u8]>,
    ) -> Lowered<Definition> {
        let (desc, extra_desc) = self.description(pipeline, attributes, command);
        let (lowered_attributes, attribute_values) = self.attributes(attributes)?;
        // A `def` that is not the keyword (an engine without the core language) is parsed like
        // any command by the classic parser.
        let decl_id = find_keyword_decl(self.working_set, b"def")
            .ok_or(Unlowered::Unsupported("def that is not a keyword"))?;
        // The call covers the command's items: from `def` (or `export`) to the body.
        let call_span = self.span(command.span);
        let head = self.command_head(command, "def");

        let (call, output) = self.in_scope(|this| {
            let mut call = Call::new(head);
            call.decl_id = decl_id;
            let _ = this.working_set.add_span(head);
            let name_span = this.span(def.name.span);
            // The name is `def`'s `string` positional (`parse_value`), which refuses a bare
            // `true`, `false` or `null` and a `{...}`; a name it reads otherwise is the classic
            // parser's to report.
            let name = this.checked(|ws| parse_value(ws, name_span, &SyntaxShape::String, None))?;
            if !matches!(&name.expr, Expr::String(s) if *s == *def.name.item) {
                return Err(Unlowered::Error);
            }
            // The flags, before or after the name, come before the signature and the body, as
            // `parse_internal_call` reads the items in order (and the name keeps no error).
            let long = |flag| match flag {
                w::DefFlag::Env => "env",
                w::DefFlag::Wrapped => "wrapped",
            };
            let mut arguments = this.definition_flags(&def.flags, long)?;
            let signature = this.signature_expression(&def.signature, false)?;
            let input_type = match &signature.expr {
                Expr::Signature(sig) => sig.get_input_type(),
                _ => Type::Any,
            };
            let body_span = match &def.body_params {
                Some(params) => this.braces_span_with_params(params, &def.body)?,
                None => this.braces_span(&def.body)?,
            };
            // `finish_def` compiles the body (`parse_def` marks it the same way). The outer
            // mark is restored before `?`: this `def` may be in another one's body, and a
            // statement handed back does not end the lowering of the statements after it.
            let def_body_span = Some(this.span(body_span));
            let outer_def_body_span =
                std::mem::replace(&mut this.working_set.def_body_span, def_body_span);
            let body = this.closure(
                body_span,
                def.body_params.as_ref(),
                &def.body,
                &SyntaxShape::Closure(None),
                Some(&input_type),
            );
            this.working_set.def_body_span = outer_def_body_span;
            let body = body?;
            // The arguments in source order.
            arguments.push((name.span.start, Argument::Positional(name)));
            arguments.push((signature.span.start, Argument::Positional(signature)));
            arguments.push((body.span.start, Argument::Positional(body)));
            arguments.sort_by_key(|(start, _)| *start);
            call.arguments = arguments
                .into_iter()
                .map(|(_, argument)| argument)
                .collect();
            let output = this.finish_keyword_call(&call, decl_id, None)?;
            Ok((call, output))
        })?;

        // `finish_def` is the classic parser's own code and commits the definition, so what it
        // reports stands; past it, only an engine without `export def` hands the statement
        // back. The call is `CallKind::Valid`: `finish_keyword_call` handed back any other.
        let (expression, decl) = finish_def(
            self.working_set,
            DefCall {
                call: Box::new(call),
                output,
                call_kind: CallKind::Valid,
                call_span,
                decl_id,
            },
            desc,
            extra_desc,
            attribute_values,
            module_name,
        );
        let expression = self.definition_expression(
            expression,
            lowered_attributes,
            command,
            attributes,
            module_name,
            "export def",
        )?;
        Ok((expression, decl))
    }

    /// `extern [--strict] name [signature]` (`parse_extern`), with its `attributes`; `command`
    /// is the `extern` expression or the `export` expression around it. As for `def`, the call
    /// (the flags, the name and the signature, whose parameters get no variables) is lowered in
    /// a scope of its own; once the scope is left, `finish_extern` makes the predeclared
    /// command a known external, strict when the call has `--strict`. Returns the
    /// definition's expression and the command's name and declaration.
    pub(super) fn extern_def(
        &mut self,
        pipeline: &w::Pipeline<'s>,
        command: &w::Expression<'s>,
        extern_def: &w::Extern<'s>,
        attributes: &[w::Attribute<'s>],
        module_name: Option<&[u8]>,
    ) -> Lowered<Definition> {
        let (desc, extra_desc) = self.description(pipeline, attributes, command);
        let (lowered_attributes, attribute_values) = self.attributes(attributes)?;
        let decl_id = find_keyword_decl(self.working_set, b"extern")
            .ok_or(Unlowered::Unsupported("extern that is not a keyword"))?;
        let call_span = self.span(command.span);
        let head = self.command_head(command, "extern");
        let name_span = self.span(extern_def.name.span);
        let call = self.in_scope(|this| {
            let mut call = Call::new(head);
            call.decl_id = decl_id;
            let _ = this.working_set.add_span(head);
            // As for `def`: the name is `extern`'s `string` positional (`parse_value`), and
            // the flags come before the signature.
            let name = this.checked(|ws| parse_value(ws, name_span, &SyntaxShape::String, None))?;
            if !matches!(&name.expr, Expr::String(s) if *s == *extern_def.name.item) {
                return Err(Unlowered::Error);
            }
            let long = |flag| match flag {
                w::ExternFlag::Strict => "strict",
            };
            let mut arguments = this.definition_flags(&extern_def.flags, long)?;
            let signature = this.signature_expression(&extern_def.signature, true)?;
            // The arguments in source order.
            arguments.push((name.span.start, Argument::Positional(name)));
            arguments.push((signature.span.start, Argument::Positional(signature)));
            arguments.sort_by_key(|(start, _)| *start);
            call.arguments = arguments
                .into_iter()
                .map(|(_, argument)| argument)
                .collect();
            this.finish_keyword_call(&call, decl_id, None)?;
            Ok(call)
        })?;
        let (expression, decl) = finish_extern(
            self.working_set,
            Box::new(call),
            call_span,
            name_span,
            desc,
            extra_desc,
            attribute_values,
            module_name,
        );
        let expression = self.definition_expression(
            expression,
            lowered_attributes,
            command,
            attributes,
            module_name,
            "export extern",
        )?;
        Ok((expression, decl))
    }

    /// The flags of a `def` or an `extern`, each lowered with its long name (`long`,
    /// [`Self::definition_flag`]).
    fn definition_flags<Flag: Copy>(
        &mut self,
        flags: &[w::DefinitionFlag<'s, Flag>],
        long: fn(Flag) -> &'static str,
    ) -> Lowered<Vec<(usize, Argument)>> {
        flags
            .iter()
            .map(|flag| self.definition_flag(flag, long(flag.flag)))
            .collect()
    }

    /// A flag of `def` or `extern`, named `long`: the named argument that the classic call
    /// parser makes of it, its value read as a `bool` (`--env=false`), which `finish_def` and
    /// `finish_extern` evaluate. Keyed by where it starts, for putting the call's arguments in
    /// source order.
    fn definition_flag<Flag>(
        &mut self,
        flag: &w::DefinitionFlag<'s, Flag>,
        long: &str,
    ) -> Lowered<(usize, Argument)> {
        let span = self.span(flag.span);
        let (name_span, value) = match &flag.value {
            Some(value) => (
                Span::new(span.start, span.start + long.len() + 2),
                Some(self.flag_value(value, &SyntaxShape::Boolean)?),
            ),
            None => (span, None),
        };
        let name = Spanned {
            item: long.to_string(),
            span: name_span,
        };
        Ok((span.start, Argument::Named((name, None, value))))
    }

    /// `const name = value` (`parse_const`): the value is evaluated now and the variable holds
    /// it. In order: the value, a pipeline without input lowered as an `Expr::Subexpression`
    /// (where `let` makes an `Expr::Block`); the variable, created after it; a written type,
    /// checked against the value's type and then the constant's, and a string held by a `glob`
    /// variable made a glob; the variable put in scope. A type that doesn't match, or a value
    /// that does not evaluate, is reported and the statement kept, the variable then holding no
    /// value. `keyword` is the `const` expression. Returns the statement's expression and the
    /// variable's name span.
    ///
    /// `const x` without a value is a plain call to `const` (`binding_without_value`), as
    /// `parse_const` reads a statement without an `=`; it gives no name span, so an `export
    /// const x` exports nothing.
    pub(super) fn const_statement(
        &mut self,
        keyword: &w::Expression<'s>,
        binding: &w::Binding<'s>,
    ) -> Lowered<(Expression, Option<Span>)> {
        let Some(value) = &binding.value else {
            let call = self.binding_without_value(keyword, binding, "const", None)?;
            return Ok((call, None));
        };
        let decl_id = self
            .working_set
            .find_decl(b"const")
            .ok_or(Unlowered::Error)?;
        let rvalue_span = self.span(value.span);
        let block = self.block(value, rvalue_span, false, BlockOf::Value, None)?;
        let rvalue_ty = block.output_type();
        let block_id = self.working_set.add_block(Arc::new(block));
        let rvalue = self.node(Expr::Subexpression(block_id), rvalue_span, rvalue_ty);
        let (lvalue, explicit_type, var_name) = self.binding_variable(binding, false, None)?;
        if let Some(explicit_type) = &explicit_type
            && !type_compatible(explicit_type, &rvalue.ty)
        {
            self.working_set.error(ParseError::TypeMismatch(
                explicit_type.clone(),
                rvalue.ty.clone(),
                rvalue_span,
            ));
        }
        if let Some(var_id) = lvalue.as_var() {
            if explicit_type.is_none() {
                self.working_set
                    .set_variable_type(var_id, rvalue.ty.clone());
            }
            match eval_constant(self.working_set, &rvalue) {
                Ok(mut value) => {
                    let mut const_type = value.get_type();
                    if let Some(explicit_type) = &explicit_type {
                        if !type_compatible(explicit_type, &const_type) {
                            self.working_set.error(ParseError::TypeMismatch(
                                explicit_type.clone(),
                                const_type.clone(),
                                rvalue_span,
                            ));
                        }
                        if let Value::String { val, .. } = &value
                            && explicit_type == &Type::Glob
                        {
                            value = Value::glob(val.clone(), false, value.span());
                            const_type = value.get_type();
                        }
                    }
                    self.working_set.set_variable_type(var_id, const_type);
                    self.working_set.set_variable_const_val(var_id, value);
                }
                Err(error) => {
                    let error = error.wrap(self.working_set, rvalue_span);
                    self.working_set.error(error);
                }
            }
            // In scope once the statement can no longer be handed back (`variable_declaration`).
            self.working_set
                .insert_variable_into_scope(var_name, var_id);
        }
        let head = self.keyword_span(keyword, "const");
        let lvalue_span = lvalue.span;
        let call = Box::new(Call {
            decl_id,
            head,
            arguments: vec![Argument::Positional(lvalue), Argument::Positional(rvalue)],
            parser_info: HashMap::new(),
        });
        let span = self.span(keyword.span);
        Ok((
            self.node(Expr::Call(call), span, Type::Any),
            Some(lvalue_span),
        ))
    }

    /// `type name = type` (`parse_type_alias`), or `export type` when `command` is an
    /// `export`: `finish_type_alias` checks the name, reads the type and defines the alias, and
    /// for `export type` the call is made a call to `export type` covering the statement
    /// (`warp_export_call`). Returns the statement and the alias, which an `export type` in a
    /// module exports, as an exportable. The head's span is registered, as `parse_internal_call` registers it for
    /// the call that checks whether the statement asks for help.
    ///
    /// A type that doesn't parse still defines the alias, as `any`, so once
    /// `finish_type_alias` has run the statement is never handed back.
    pub(super) fn type_alias(
        &mut self,
        command: &w::Expression<'s>,
        alias: &w::TypeAlias<'s>,
    ) -> Lowered<(Pipeline, Vec<Exportable>)> {
        let export = matches!(command.expr, w::Expr::Export(_));
        let decl_id = find_keyword_decl(self.working_set, b"type")
            .ok_or(Unlowered::Unsupported("type that is not a keyword"))?;
        if export && find_keyword_decl(self.working_set, b"export type").is_none() {
            return Err(Unlowered::Unsupported("export type that is not a keyword"));
        }
        let head = self.command_head(command, "type");
        let _ = self.working_set.add_span(head);
        let call_span = self.span(command.span);
        let (mut pipeline, type_alias) = finish_type_alias(
            self.working_set,
            decl_id,
            head,
            call_span,
            self.span(alias.name.span),
            self.span(alias.eq),
            // No type after the `=`: the winnow parser's type is a point after it.
            Some(self.span(alias.ty.span)).filter(|span| !span.is_empty()),
        );
        if export {
            // `export`, `type`, and the rest of the statement, as `warp_export_call` takes them.
            let spans = [
                Span::new(head.start, head.start + "export".len()),
                Span::new(head.end - "type".len(), head.end),
                Span::new(call_span.end, call_span.end),
            ];
            warp_export_call(self.working_set, &mut pipeline, "export type", &spans);
        }
        let exportables = type_alias
            .map(|(name, id)| Exportable::TypeAlias { name, id })
            .into_iter()
            .collect();
        Ok((pipeline, exportables))
    }

    /// A statement keyword's call for help (`def foo --help`), `call`, and `command` the
    /// statement: the call itself, or the `export` around it, in the body of the module
    /// `module_name` for an exported definition. The statement is the call
    /// the keyword's parse function makes to see whether the statement asks for help, its
    /// arguments read by the keyword's signature (`parse_internal_call`), as that function makes
    /// it:
    ///
    /// - `def` and `extern` read it in a scope of their own, `def` with its last item marked as
    ///   its body, and finish it with `finish_def` (which drops the predeclaration) and
    ///   `finish_extern` (which defines the command all the same), whatever its kind: the
    ///   winnow parser also gives a `def` or `extern` whose call has an error of its own as a
    ///   call (`def --foo f [] {}`), which defines the command with its description;
    /// - `for` reads it in a scope of its own;
    /// - `type` reads only its first two items;
    /// - `let` reads it with the statement's input;
    /// - an `export` makes it a call to `export <keyword>` covering the statement
    ///   (`warp_export_call`).
    ///
    /// (`export-env` reads only its first item, as the winnow parser's call already has it.)
    /// `alias` and `type` drop the errors of the call, which `internal_call` hands back.
    ///
    /// Returns the statement and, for `def` and `extern`, the command it defined, which an
    /// `export` in a module exports. Another keyword's call that does not ask for help, a
    /// redirected statement
    /// (an error for these keywords but `where`, which keeps it, and `const`, which drops it)
    /// are the classic parser's.
    pub(super) fn help_statement(
        &mut self,
        pipeline: &w::Pipeline<'s>,
        command: &w::Expression<'s>,
        call: &w::Call<'s>,
        module_name: Option<&[u8]>,
        input_type: Option<&Type>,
    ) -> Lowered<Definition> {
        let keyword = call.head.name.as_ref();
        if pipeline
            .elements
            .iter()
            .any(|element| element.redirection.is_some())
        {
            return Err(match keyword {
                "where" | "const" => Unlowered::Unsupported("redirected call for help"),
                _ => Unlowered::Error,
            });
        }
        if keyword == "where" {
            return Ok((self.where_call(command, call)?, None));
        }
        let export = matches!(command.expr, w::Expr::Export(_));
        // `parse_alias` checks where its items stand before anything else, a call for help
        // included, and gives garbage over the misplaced one.
        // `export` and `alias` and the items after them, as `parse_alias` takes them.
        let mut alias_spans = Vec::new();
        if keyword == "alias" {
            let keyword_start = match &command.expr {
                w::Expr::Export(export) => self.span(export.item.span).start,
                _ => self.span(call.head.span).start,
            };
            let start = self.span(command.span).start;
            let spans: Vec<Span> = (export.then(|| Span::new(start, start + "export".len())))
                .into_iter()
                .chain(Some(Span::new(
                    keyword_start,
                    keyword_start + "alias".len(),
                )))
                .chain(
                    call.arguments
                        .iter()
                        .map(|argument| self.span(argument.span())),
                )
                .collect();
            let errors = self.working_set.parse_errors.len();
            if let Some(span) = check_alias_name(self.working_set, &spans) {
                // An export makes the garbage its own and reports it (`warp_export_call`).
                if export {
                    self.working_set.parse_errors.truncate(errors);
                    return Err(Unlowered::Unsupported("export of a misplaced alias"));
                }
                return Ok((garbage(self.working_set, *span), None));
            }
            alias_spans = spans;
        }
        let decl_id = find_keyword_decl(self.working_set, keyword.as_bytes())
            .ok_or(Unlowered::Unsupported("keyword command"))?;
        let export_name = format!("export {keyword}");
        if export && find_keyword_decl(self.working_set, export_name.as_bytes()).is_none() {
            return Err(Unlowered::Unsupported("export that is not a keyword"));
        }
        let head = self.command_head(command, keyword);
        let call_span = self.span(command.span);
        let arguments = call.arguments.as_slice();
        // `parse_export_env` reports a lone `export-env` before reading its call, with garbage.
        if keyword == "export-env" && arguments.is_empty() {
            self.working_set.error(ParseError::MissingPositional(
                "block".into(),
                call_span,
                "export-env <block>".into(),
            ));
            return Ok((garbage(self.working_set, call_span), None));
        }
        // `parse_def` and `parse_extern` refuse a name with its parameters joined to it, in
        // the definition's scope, which they enter and leave open, and give garbage.
        let name_span = arguments
            .iter()
            .map(w::Argument::span)
            .find(|span| !self.text(*span).starts_with('-'));
        if matches!(keyword, "def" | "extern")
            && let Some(name_span) = name_span
            && let Some(error) =
                detect_params_in_name(self.working_set, self.span(name_span), decl_id)
        {
            self.working_set.enter_scope();
            self.working_set.error(error);
            // An export of that garbage is no call (`warp_export_call`).
            if let w::Expr::Export(export) = &command.expr {
                let keyword = self.span(export.item.span);
                self.working_set.error(ParseError::InternalError(
                    "unexpected output from parsing a definition".into(),
                    Span::new(keyword.start, call_span.end),
                ));
            }
            return Ok((garbage(self.working_set, call_span), None));
        }
        let (lowered, output, kind) = match keyword {
            "def" => self.in_scope(|this| {
                let body = arguments.last().map(|argument| this.span(argument.span()));
                let outer = std::mem::replace(&mut this.working_set.def_body_span, body);
                let lowered = this.internal_call(head, arguments, decl_id, None);
                this.working_set.def_body_span = outer;
                lowered
            })?,
            "extern" | "for" => {
                self.in_scope(|this| this.internal_call(head, arguments, decl_id, None))?
            }
            // `parse_alias` and `parse_type_alias` drop the errors of the call.
            "alias" | "type" => {
                let errors = self.working_set.parse_errors.len();
                let arguments = match keyword {
                    "type" => &arguments[..arguments.len().min(2)],
                    _ => arguments,
                };
                let lowered = self.internal_call(head, arguments, decl_id, None)?;
                self.working_set.parse_errors.truncate(errors);
                lowered
            }
            "let" => self.internal_call(head, arguments, decl_id, input_type)?,
            _ => self.internal_call(head, arguments, decl_id, None)?,
        };
        // `parse_alias` keeps a call to `alias` that declares nothing (`alias ll ls`, `alias ll
        // =`): a name it refuses is garbage (`alias_name`), anything else an incomplete alias.
        if keyword == "alias" && kind != CallKind::Help {
            let Some(name) = lowered.positional_iter().next() else {
                return Err(Unlowered::Unsupported("alias call without a name"));
            };
            if alias_name(self.working_set, name).is_none() {
                if export {
                    return Err(Unlowered::Unsupported("export of a refused alias name"));
                }
                return Ok((garbage(self.working_set, call_span), None));
            }
            // In a module's body `parse_export_in_module` gives `parse_alias` the statement
            // without its `export`.
            let args = if export { 2 } else { 1 };
            let command = if export && module_name.is_some() {
                1
            } else {
                0
            };
            report_incomplete_alias(
                self.working_set,
                &alias_spans[command..args],
                &alias_spans[args..],
            );
        }
        // `parse_type_alias` reports items out of place, which the winnow parser read as the
        // call, with garbage over the statement.
        if keyword == "type" && kind != CallKind::Help {
            let spans: Vec<Span> = arguments
                .iter()
                .map(|argument| self.span(argument.span()))
                .collect();
            if let Some(error) = type_alias_layout_error(self.working_set, head, &spans) {
                self.working_set.error(error);
                let garbage = garbage(self.working_set, call_span);
                return Ok(match &command.expr {
                    w::Expr::Export(export) => (
                        self.exported_garbage(garbage, export, call_span, call_span),
                        None,
                    ),
                    _ => (garbage, None),
                });
            }
        }
        // `parse_def` and `parse_extern` finish any call: one with an error of its own (the
        // winnow parser's call form of `def --foo f [] {}`) still defines the command. A
        // binding without a value is its call whatever its kind (`let x --foo`).
        // `parse_export_env` and `parse_use` give an invalid call back as it is (`export-env
        // --foo`, `use`), and `parse_for` finishes any call (`finish_for`).
        if kind != CallKind::Help
            && !matches!(
                keyword,
                "def" | "extern" | "for" | "alias" | "let" | "mut" | "const"
            )
            && !(matches!(keyword, "export-env" | "use") && kind == CallKind::Invalid)
        {
            return Err(Unlowered::Unsupported(
                "keyword call that does not ask for help",
            ));
        }
        let (mut expression, decl) = match keyword {
            "def" => {
                let (desc, extra_desc) = self.description(pipeline, &[], command);
                finish_def(
                    self.working_set,
                    DefCall {
                        call: lowered,
                        output,
                        call_kind: kind,
                        call_span,
                        decl_id,
                    },
                    desc,
                    extra_desc,
                    Vec::new(),
                    module_name,
                )
            }
            "extern" => {
                let (desc, extra_desc) = self.description(pipeline, &[], command);
                let name_span = name_span.map_or(call_span, |span| self.span(span));
                finish_extern(
                    self.working_set,
                    lowered,
                    call_span,
                    name_span,
                    desc,
                    extra_desc,
                    Vec::new(),
                    module_name,
                )
            }
            "for" => (
                finish_for(self.working_set, lowered, output, kind, call_span, decl_id),
                None,
            ),
            _ => (self.node(Expr::Call(lowered), call_span, output), None),
        };
        if let w::Expr::Export(export) = &command.expr {
            if matches!(keyword, "def" | "extern") && !matches!(expression.expr, Expr::Call(_)) {
                let expression = self.exported_garbage(expression, export, call_span, call_span);
                return Ok((expression, decl));
            }
            self.export_call(&mut expression, &export_name, call_span)?;
            if let Expr::Call(call) = &mut expression.expr {
                call.head = head;
            }
        }
        Ok((expression, decl))
    }

    /// `alias name = target` (`parse_alias`), in the module `module_name` when it is exported
    /// from one, whose name the alias may not have. The statement is a call to `alias` whose `=`
    /// argument is the target read as an expression, with the predeclared commands of the block
    /// visible (`parse_internal_call`), and whose errors are dropped. The alias's body is the
    /// target read again as a call with them hidden (`search_predecls`), so a command defined
    /// later in the block is an external one there; of its errors, missing arguments are
    /// forgiven (`alias e = enter`). `finish_alias` checks it and declares the alias. Like the
    /// classic parser, which parses the target twice, the lowering lowers it twice.
    pub(super) fn alias(
        &mut self,
        pipeline: &w::Pipeline<'s>,
        command: &w::Expression<'s>,
        alias: &w::Alias<'s>,
        module_name: Option<&[u8]>,
    ) -> Lowered<Expression> {
        let decl_id = find_keyword_decl(self.working_set, b"alias")
            .ok_or(Unlowered::Unsupported("alias that is not a keyword"))?;
        let errors_before = self.working_set.parse_errors.len();
        let head = self.command_head(command, "alias");
        let mut call = Call::new(head);
        call.decl_id = decl_id;
        let _ = self.working_set.add_span(head);
        let name_span = self.span(alias.name.span);
        // Its errors are the call's, dropped below.
        let name = parse_value(self.working_set, name_span, &SyntaxShape::String, None);
        let target = alias.value.as_ref();
        // Read as an expression, a target starting with `NAME=value` sets an environment
        // variable for the rest (a call to `with-env`): the winnow parser read the target as a
        // call, the body's reading, and reads it again for this one.
        let target_text = self.text(target.span);
        let starts_with_shorthand = target_text
            .split_ascii_whitespace()
            .next()
            .and_then(|word| Some((word, word.find('=')?)))
            .is_some_and(|(word, equals)| is_env_variable_name(&word.as_bytes()[..equals]));
        // Read as an expression, a target with an assignment operator is an assignment
        // (`parse_assignment_expression`), whose errors the classic parser drops.
        if target_text
            .split_ascii_whitespace()
            .any(|word| is_assignment_operator(word.as_bytes()))
        {
            return Err(Unlowered::Unsupported("alias of an assignment"));
        }
        // A lone `if` or `match` is a math expression (`is_math_expression_like`), which
        // `parse_math_expression` cannot read.
        let value = if matches!(target_text, "if" | "match") {
            let span = self.span(target.span);
            Ok(garbage(self.working_set, span))
        } else if starts_with_shorthand {
            self.reread(target.span, ItemsValue::AliasTarget, None)
        } else if let w::Expr::Garbage = target.expr {
            // A target that reads as an expression, which the winnow parser left as garbage:
            // the argument is what `parse_expression` reads (`alias a = not` is garbage). One
            // nu does not read so (`alias a = 0..$x.len`) is an alias, the classic parser's.
            let items = self.items(target.span);
            if !items
                .first()
                .is_some_and(|first| is_math_expression_like(self.working_set, *first))
            {
                return Err(Unlowered::Unsupported("alias target read as a call"));
            }
            Ok(parse_expression(self.working_set, &items, None))
        } else {
            self.lower_alias_target(target)
        };
        // The errors of the call to `alias` are dropped.
        self.working_set.parse_errors.truncate(errors_before);
        let value = value?;
        let equals = self.span(alias.eq);
        let span = equals.merge(value.span);
        let ty = value.ty.clone();
        let keyword = Keyword {
            keyword: b"=".as_slice().into(),
            span: equals,
            expr: value,
        };
        call.arguments = vec![
            Argument::Positional(name),
            Argument::Positional(self.node(Expr::Keyword(Box::new(keyword)), span, ty)),
        ];
        let output = self.finish_keyword_call(&call, decl_id, None)?;
        let statement_span = self.span(command.span);
        let statement = self.node(Expr::Call(Box::new(call.clone())), statement_span, output);

        let Some(name) = call
            .positional_iter()
            .next()
            .and_then(|name| alias_name(self.working_set, name))
        else {
            return Ok(garbage(self.working_set, statement_span));
        };
        if !alias_name_fits_module(self.working_set, &name, module_name, name_span) {
            return Ok(statement);
        }
        // A target that reads as an expression (`alias a = 1`), which the winnow parser left as
        // garbage: `CantAliasExpression`, naming what `parse_expression` reads, and no alias.
        if let w::Expr::Garbage = target.expr
            && let items = self.items(target.span)
            && let Some(first) = items.first().copied()
        {
            let errors = self.working_set.parse_errors.len();
            let expression = parse_expression(self.working_set, &items, None);
            self.working_set.parse_errors.truncate(errors);
            self.working_set.error(ParseError::CantAliasExpression(
                expression.expr.description().to_string(),
                first,
            ));
            return Ok(statement);
        }
        let outer_search_predecls = std::mem::replace(&mut self.working_set.search_predecls, false);
        let wrapped_call = self.lower_alias_target(target);
        self.working_set.search_predecls = outer_search_predecls;
        let wrapped_call = wrapped_call?;
        // When the first error is a missing argument, all are forgiven; any other makes the
        // statement garbage, its errors kept. They come from the target's nested statements,
        // which the classic parser parses when the lowering gives them back.
        match self.working_set.parse_errors.get(errors_before) {
            None => {}
            Some(error) if is_missing_argument(error) => {
                self.working_set.parse_errors.truncate(errors_before);
            }
            Some(_) => {
                let span = self.span(target.span);
                return Ok(garbage(self.working_set, span));
            }
        }
        let comments = self.doc_comments(pipeline, &[], command);
        finish_alias(self.working_set, &call, name, wrapped_call, &comments);
        Ok(statement)
    }

    /// An alias's target, read as `parse_call` reads it (`alias_target`).
    fn lower_alias_target(&mut self, target: &w::Expression<'s>) -> Lowered<Expression> {
        let outer = std::mem::replace(&mut self.alias_target, true);
        let expression = self.expression(target, None);
        self.alias_target = outer;
        expression
    }

    pub(super) fn export_env(
        &mut self,
        e: &w::Expression<'s>,
        export_env: &w::ExportEnv<'s>,
    ) -> Lowered<(Expression, nu_protocol::BlockId)> {
        let (mut call, decl_id) = self.keyword_call(e, "export-env")?;
        let signature = self.working_set.get_signature_shared(decl_id);
        if signature.creates_scope {
            self.working_set.enter_scope();
        }
        let block = self.block_argument(&export_env.body, None, None);
        if signature.creates_scope {
            self.working_set.exit_scope();
        }
        let block = block?;
        let block_id = block.as_block().ok_or(Unlowered::Error)?;
        call.add_positional(block);
        self.finish_keyword_call(&call, decl_id, None)?;
        let span = self.span(e.span);
        let expression = self.node(Expr::Call(Box::new(call)), span, Type::Any);
        compile_block_with_id(self.working_set, block_id);
        Ok((expression, block_id))
    }

    /// A definition's description and extra description, from its doc comments and the comment
    /// after it on its line (`build_desc` over the lite command's comments). With attributes
    /// (`command` is the definition after them), the comments are those the classic lite parser
    /// keeps: a comment beside an attribute drops the whole-line comments before it, so the doc
    /// comments are the whole-line comments after the last comment beside an attribute (all of
    /// them when no attribute has one) or, when that leaves none, the comments beside the
    /// attributes. Comments from the definition on are kept in any case.
    fn description(
        &mut self,
        pipeline: &w::Pipeline<'s>,
        attributes: &[w::Attribute<'s>],
        command: &w::Expression<'s>,
    ) -> (String, String) {
        let comments = self.doc_comments(pipeline, attributes, command);
        self.working_set.build_desc(&comments)
    }

    /// The comments the classic lite parser keeps with `command` (`LiteCommand::comments`), the
    /// statement of `pipeline` after `attributes`: see [`Self::description`].
    pub(super) fn doc_comments(
        &self,
        pipeline: &w::Pipeline<'s>,
        attributes: &[w::Attribute<'s>],
        command: &w::Expression<'s>,
    ) -> Vec<Span> {
        let mut comments: Vec<WSpan> = pipeline
            .leading_comments
            .iter()
            .chain(&pipeline.trailing_comments)
            .map(|comment| comment.span)
            .collect();
        if let Some(first) = attributes.first() {
            comments.sort_by_key(|span| span.start);
            // `after`: from the definition on; `beside`: after an attribute on its line, which
            // drops the whole-line comments before it; `lines`: whole-line comments.
            let (mut lines, mut beside, mut after) = (Vec::new(), Vec::new(), Vec::new());
            for span in comments {
                if span.start > command.span.start {
                    after.push(span);
                } else if span.start > first.span.start && !self.starts_line(span.start) {
                    beside.push(span);
                    lines.clear();
                } else {
                    lines.push(span);
                }
            }
            comments = if lines.is_empty() { beside } else { lines };
            comments.extend(after);
        }
        comments.into_iter().map(|span| self.span(span)).collect()
    }

    /// Whether only spaces or tabs come before `offset` on its line.
    fn starts_line(&self, offset: usize) -> bool {
        let before = self.source[..offset].trim_end_matches([' ', '\t', '\r']);
        before.is_empty() || before.ends_with('\n')
    }

    /// A definition's attributes and their constant values (the loop that starts `parse_def`
    /// and `parse_extern`). An attribute that does not evaluate to a constant is reported and
    /// kept, without a value.
    fn attributes(&mut self, attributes: &[w::Attribute<'s>]) -> Lowered<LoweredAttributes> {
        let mut lowered = Vec::with_capacity(attributes.len());
        let mut values = Vec::with_capacity(attributes.len());
        for attribute in attributes {
            let (attribute, name) = self.attribute(attribute)?;
            if let Some(name) = name {
                match eval_constant(self.working_set, &attribute.expr) {
                    Ok(value) => values.push((name, value)),
                    Err(error) => {
                        let error = error.wrap(self.working_set, attribute.expr.span);
                        self.working_set.error(error);
                    }
                }
            }
            lowered.push(attribute);
        }
        Ok((lowered, values))
    }

    /// Attributes with no definition after them (`parse_attribute_block`): each read as
    /// `parse_attribute` reads it, `AttributeRequiresDefinition` at the last one, and the
    /// command after them (`command`, garbage: the point after the last attribute when there
    /// is none) kept as the block's item.
    pub(super) fn attributes_without_definition(
        &mut self,
        command: &w::Expression<'s>,
        attributes: &[w::Attribute<'s>],
    ) -> Lowered<Expression> {
        let mut lowered = Vec::with_capacity(attributes.len());
        for attribute in attributes {
            lowered.push(self.attribute(attribute)?.0);
        }
        let (Some(first), Some(last)) = (lowered.first(), lowered.last()) else {
            return Err(Unlowered::Error);
        };
        let first = first.expr.span;
        self.working_set
            .error(ParseError::AttributeRequiresDefinition(last.expr.span));
        let command_span = self.span(command.span);
        let item = Expression::garbage(self.working_set, command_span);
        let ty = item.ty.clone();
        let span = Span::merge_many([first, command_span]);
        Ok(self.node(
            Expr::AttributeBlock(AttributeBlock {
                attributes: lowered,
                item: Box::new(item),
            }),
            span,
            ty,
        ))
    }

    /// The head of a definition's call: `keyword`, or `export keyword` when `command` is an
    /// `export`, covering what comes between the two words (blanks, and in a subexpression
    /// newlines and comments), as `Span::concat` of the two items does.
    pub(super) fn command_head(&self, command: &w::Expression<'s>, keyword: &str) -> Span {
        let start = self.span(command.span).start;
        match &command.expr {
            w::Expr::Export(export) => {
                Span::new(start, self.span(export.item.span).start + keyword.len())
            }
            _ => Span::new(start, start + keyword.len()),
        }
    }

    /// A definition's expression with its attributes around it (`parse_def`, `parse_extern`),
    /// made a call to `export_name` when `command` is an `export` (`warp_export_call`).
    fn definition_expression(
        &mut self,
        mut expression: Expression,
        lowered_attributes: Vec<Attribute>,
        command: &w::Expression<'s>,
        attributes: &[w::Attribute<'s>],
        module_name: Option<&[u8]>,
        export_name: &str,
    ) -> Lowered<Expression> {
        if !lowered_attributes.is_empty() {
            let span = Span::merge_many(
                lowered_attributes
                    .first()
                    .map(|attribute| attribute.expr.span)
                    .into_iter()
                    .chain(Some(expression.span)),
            );
            let ty = expression.ty.clone();
            expression = self.node(
                Expr::AttributeBlock(AttributeBlock {
                    attributes: lowered_attributes,
                    item: Box::new(expression),
                }),
                span,
                ty,
            );
        }
        if let w::Expr::Export(export) = &command.expr {
            let call_span = self.span(command.span);
            // In a module the definition covers the command's items; elsewhere every item of
            // the statement, its attributes included.
            let first_attribute = attributes
                .first()
                .map(|attribute| self.span(attribute.span));
            let span = match (module_name, first_attribute) {
                (None, Some(first)) => Span::new(first.start, call_span.end),
                _ => call_span,
            };
            if attributes.is_empty() && !matches!(expression.expr, Expr::Call(_)) {
                return Ok(self.exported_garbage(expression, export, span, call_span));
            }
            self.export_call(&mut expression, export_name, span)?;
        }
        Ok(expression)
    }

    /// An exported definition `finish_def` or `finish_extern` gave up on (garbage: a flag whose
    /// value does not evaluate, a name that is no string): kept, covering `span`, and reported
    /// from the keyword to the end of the statement at `call_span` (`warp_export_call`).
    fn exported_garbage(
        &mut self,
        mut expression: Expression,
        export: &w::Export<'s>,
        span: Span,
        call_span: Span,
    ) -> Expression {
        expression.span = span;
        let keyword = self.span(export.item.span).start;
        self.working_set.error(ParseError::InternalError(
            "unexpected output from parsing a definition".into(),
            Span::new(keyword, call_span.end),
        ));
        expression
    }

    /// Make a definition's call a call to `export_name` covering `span` (`warp_export_call`):
    /// the call's head already covers `export` and the keyword.
    pub(super) fn export_call(
        &mut self,
        expression: &mut Expression,
        export_name: &str,
        span: Span,
    ) -> Lowered<()> {
        let export_decl = self
            .working_set
            .find_decl(export_name.as_bytes())
            .ok_or(Unlowered::Error)?;
        expression.span = span;
        match &mut expression.expr {
            Expr::Call(call) => call.decl_id = export_decl,
            Expr::AttributeBlock(block) => match &mut block.item.expr {
                Expr::Call(call) => call.decl_id = export_decl,
                _ => return Err(Unlowered::Error),
            },
            _ => return Err(Unlowered::Error),
        }
        Ok(())
    }

    /// An attribute, `@name arguments` (`parse_attribute`): a call to the command `attr name`,
    /// covering the attribute without its `@`. Returns it with its name. An unknown name, or an
    /// alias of an external command, which is not a constant command, is reported, and the
    /// attribute is garbage without a name.
    fn attribute(&mut self, attribute: &w::Attribute<'s>) -> Lowered<(Attribute, Option<String>)> {
        let name = attribute.name.item.to_string();
        // The name without the attribute's `@`; a second one (`@@name`, a call to `attr @name`)
        // stays, as `parse_attribute` drops only the first.
        let name_span = self.span(attribute.name.span);
        let span = self.span(attribute.span);
        let span = Span::new(span.start + 1, span.end);
        let error = match self
            .working_set
            .find_decl(format!("attr {name}").as_bytes())
        {
            None => ParseError::UnknownCommand(name_span),
            Some(decl_id) => match self.working_set.get_decl(decl_id).as_alias() {
                Some(alias) if matches!(alias.wrapped_call.expr, Expr::ExternalCall(..)) => {
                    ShellError::NotAConstCommand { span: name_span }.wrap(self.working_set, span)
                }
                _ => {
                    let (call, output, _) =
                        self.internal_call(name_span, &attribute.arguments, decl_id, None)?;
                    let expr = self.node(Expr::Call(call), span, output);
                    return Ok((Attribute { expr }, Some(name)));
                }
            },
        };
        self.working_set.error(error);
        let expr = Expression::garbage(self.working_set, span);
        Ok((Attribute { expr }, None))
    }
}

/// A definition's expression, and the name and declaration of the command it defined.
pub(super) type Definition = (Expression, Option<(Vec<u8>, DeclId)>);

/// The shape of the value after `keyword` (`catch`, `finally`) in `try`'s signature: an
/// optional positional of `Keyword` shapes, maybe one of several.
fn handler_shape(signature: &nu_protocol::Signature, keyword: &str) -> Option<SyntaxShape> {
    signature.optional_positional.iter().find_map(|positional| {
        let shapes = match &positional.shape {
            SyntaxShape::OneOf(shapes) => shapes.as_slice(),
            shape => std::slice::from_ref(shape),
        };
        shapes.iter().find_map(|shape| match shape {
            SyntaxShape::Keyword(word, value) if word.as_slice() == keyword.as_bytes() => {
                Some((**value).clone())
            }
            _ => None,
        })
    })
}

/// A definition's attributes, and their names with their constant values.
type LoweredAttributes = (Vec<Attribute>, Vec<(String, Value)>);
