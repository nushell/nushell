//! Keyword statements: `if`, `match`, `while`, `loop`, `try`, `return`, `break`, `continue`
//! (ordinary calls to keyword commands in the classic parser), and `let`, `mut`, `const`, `for`,
//! `def`, `extern` and `export-env` (the classic parser's `parse_let`, `parse_mut`,
//! `parse_const`, `parse_for`, `parse_def`, `parse_extern` and `parse_export_env`), with the
//! attributes and the `export` around a definition.

use std::{collections::HashMap, sync::Arc};

use nu_protocol::eval_const::eval_constant;
use nu_protocol::{
    CompileError, DeclId, PositionalArg, Span, Spanned, SyntaxShape, Type, TypeSet, Value,
    ast::{
        Argument, Attribute, AttributeBlock, Call, Expr, Expression, Keyword, MatchPattern,
        Pattern, Pipeline,
    },
};
use nu_winnow_parser::{Span as WSpan, ast as w};

use super::{Lower, Lowered, Unlowered};
use crate::{
    lex::lex,
    parse_calls::{CallKind, check_call},
    parse_captures_compile::compile_block_with_id,
    parse_def::{DefCall, finish_def, finish_extern},
    parse_keywords::find_keyword_decl,
    parse_patterns::parse_pattern,
    parse_shape_specs::parse_type,
    parse_signatures::ensure_not_reserved_variable_name,
    type_check::{check_pipeline_type, type_compatible},
};

impl<'s> Lower<'_, '_, 's> {
    /// The call to keyword `name` that `e` is, its head the keyword. The head's span is
    /// registered in the working set, as `parse_internal_call` registers it (as do the other
    /// calls the lowering builds).
    fn keyword_call(&mut self, e: &w::Expression<'s>, name: &str) -> Lowered<(Call, DeclId)> {
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
        let kind = self.checked(|ws| check_call(ws, call.head, &signature, call))?;
        if kind == CallKind::Invalid {
            return Err(Unlowered::Error);
        }
        Ok(signature
            .get_output_type(
                input_type
                    .map(|ty| ty.clone().union(Type::Nothing))
                    .as_ref(),
            )
            .unwrap_or(Type::Error))
    }

    /// A block argument of a keyword, `{ ... }` (`parse_block_expression`). A variable or
    /// subexpression in its place (`if $c $env.f`), which the classic parser type-checks as a
    /// block, is left to the classic parser.
    fn block_argument(
        &mut self,
        block: &w::Block<'s>,
        value: Option<&w::Expression<'s>>,
        input_type: Option<&Type>,
    ) -> Lowered<Expression> {
        if value.is_some() {
            return Err(Unlowered::Unsupported("value as a keyword's block"));
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

    /// `if condition { then } else { otherwise }`: a call to `if`, whose output is the union of
    /// its branches' outputs (with `nothing` when there is no `else`), as `parse_internal_call`
    /// overrides the output of `if`. The condition gets no input, the branches `input_type`.
    /// The `else` branch, a block or an expression (`else if ...`), is a `Keyword` argument.
    pub(super) fn if_call(
        &mut self,
        e: &w::Expression<'s>,
        if_expr: &w::If<'s>,
        input_type: Option<&Type>,
    ) -> Lowered<Expression> {
        let (mut call, decl_id) = self.keyword_call(e, "if")?;
        let condition = self.math(&if_expr.condition, None)?;
        call.add_positional(condition);
        let then = self.block_argument(
            &if_expr.then_block,
            if_expr.then_value.as_deref(),
            input_type,
        )?;
        let mut output = self.branch_type(&then, input_type);
        call.add_positional(then);
        if let Some(branch) = &if_expr.else_branch {
            let body = match &branch.body.expr {
                w::Expr::Block(block) => {
                    self.block_expression(branch.body.span, block, input_type)?
                }
                _ => self.expression(&branch.body, input_type)?,
            };
            output = output.union(self.branch_type(&body, input_type));
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
            output = output.union(Type::Nothing);
        }
        // For its check: the output is the branches' union, not the signature's.
        self.finish_keyword_call(&call, decl_id, input_type)?;
        let span = self.span(e.span);
        Ok(self.node(Expr::Call(Box::new(call)), span, output))
    }

    /// The type a branch of `if` or `match` gives: its block's output, or the input for an
    /// empty block (as `parse_internal_call` and `parse_match_block_expression` compute it).
    fn branch_type(&self, branch: &Expression, input_type: Option<&Type>) -> Type {
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
    /// counted before each handler.
    pub(super) fn try_call(
        &mut self,
        e: &w::Expression<'s>,
        try_expr: &w::Try<'s>,
        input_type: Option<&Type>,
    ) -> Lowered<Expression> {
        let (mut call, decl_id) = self.keyword_call(e, "try")?;
        let body = self.block_argument(&try_expr.body, try_expr.body_value.as_deref(), None)?;
        call.add_positional(body);
        for handler in &try_expr.handlers {
            let keyword = match handler.kind {
                w::HandlerKind::Catch => "catch",
                w::HandlerKind::Finally => "finally",
            };
            let compile_errors = self.working_set.compile_errors.len();
            let closure = match &handler.body.expr {
                w::Expr::Closure(closure) => self.closure(
                    handler.body.span,
                    closure.params.as_ref(),
                    &closure.body,
                    &SyntaxShape::Closure(None),
                    None,
                )?,
                _ => return Err(Unlowered::Unsupported("handler that is not a closure")),
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
    /// the arms' outputs (with `nothing` when no arm matches everything). A `{ ... }` that is a
    /// closure or a record rather than arms is left to the classic parser.
    pub(super) fn match_call(
        &mut self,
        e: &w::Expression<'s>,
        match_expr: &w::Match<'s>,
        input_type: Option<&Type>,
    ) -> Lowered<Expression> {
        if match_expr.value_block.is_some() {
            return Err(Unlowered::Unsupported("match with a value for its block"));
        }
        let (mut call, decl_id) = self.keyword_call(e, "match")?;
        let value = self.value(&match_expr.value, &SyntaxShape::Any, None)?;
        call.add_positional(value);
        let mut arms = Vec::with_capacity(match_expr.arms.len());
        let mut output = Type::one_of([]);
        for arm in &match_expr.arms {
            let (pattern, result) = self.in_scope(|this| {
                let mut pattern = this.pattern(&arm.pattern)?;
                if let Some(guard) = &arm.guard {
                    pattern.guard = Some(Box::new(this.math(guard, None)?));
                }
                let result = this.match_result(&arm.body, input_type)?;
                Ok((pattern, result))
            })?;
            output = output.union(self.branch_type(&result, input_type));
            arms.push((pattern, result));
        }
        if !arms.iter().any(|(pattern, _)| pattern.is_wildcard()) {
            output = output.union(Type::Nothing);
        }
        let block_span = self.span(match_expr.block_span);
        let block = self.node(Expr::MatchBlock(arms), block_span, output.clone());
        call.add_positional(block);
        // For its check: the output is the arms' union, not the signature's.
        self.finish_keyword_call(&call, decl_id, input_type)?;
        let span = self.span(e.span);
        Ok(self.node(Expr::Call(Box::new(call)), span, output))
    }

    /// A match arm's pattern, read by the classic pattern parser (`parse_pattern`), which also
    /// declares its variables. The alternatives of an or-pattern (`a | b`) are read one by one,
    /// as `parse_match_block_expression` reads them.
    fn pattern(&mut self, pattern: &w::MatchPattern<'s>) -> Lowered<MatchPattern> {
        match &pattern.pattern {
            w::Pattern::Or(alternatives) => {
                let mut out = Vec::with_capacity(alternatives.len());
                for alternative in alternatives {
                    out.push(self.pattern(alternative)?);
                }
                let (Some(first), Some(last)) = (out.first(), out.last()) else {
                    return Err(Unlowered::Error);
                };
                let span = Span::new(first.span.start, last.span.end);
                Ok(MatchPattern {
                    pattern: Pattern::Or(out),
                    guard: None,
                    span,
                })
            }
            _ => {
                let span = self.span(pattern.span);
                self.checked(|working_set| parse_pattern(working_set, span))
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

    /// The variable of `let`, `mut`, `const` or `for` (`parse_var_with_opt_type`), declared
    /// with its written type, or else with `input_type` (`any` without one), which the caller
    /// may replace. Returns its `VarDecl` expression and the written type.
    fn variable_declaration(
        &mut self,
        name: &nu_winnow_parser::Spanned<&'s str>,
        ty: Option<&w::TypeAnnotation<'s>>,
        mutable: bool,
        input_type: Option<&Type>,
    ) -> Lowered<(Expression, Option<Type>)> {
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
        self.checked(|ws| ensure_not_reserved_variable_name(ws, &var_name, name_span))?;
        match ty {
            Some(annotation) => {
                let span = self.span(annotation.span);
                let text = self.text(annotation.span).as_bytes();
                let ty = self.checked(|ws| parse_type(ws, text, span))?;
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
                let var_id = self
                    .working_set
                    .add_variable(var_name, var_span, ty.clone(), mutable);
                let decl = self.node(Expr::VarDecl(var_id), name_span, ty.clone());
                Ok((decl, Some(ty)))
            }
            None => {
                let var_id = self.working_set.add_variable(
                    var_name,
                    name_span,
                    input_type.cloned().unwrap_or(Type::Any),
                    mutable,
                );
                let decl = self.node(Expr::VarDecl(var_id), name_span, Type::Any);
                Ok((decl, None))
            }
        }
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
    /// 2. The variable, declared with its written type (an untyped `let` variable with
    ///    `input_type` until step 3).
    /// 3. A written type must accept the value's type; an untyped variable takes the value's.
    ///
    /// `let x` without a value is left to the classic parser, which reads it as a plain call.
    fn binding(
        &mut self,
        element: &w::PipelineElement<'s>,
        binding: &w::Binding<'s>,
        keyword: &str,
        mutable: bool,
        input_type: Option<&Type>,
    ) -> Lowered<Pipeline> {
        let Some(value) = &binding.value else {
            return Err(Unlowered::Unsupported("binding without a value"));
        };
        let decl_id = self
            .working_set
            .find_decl(keyword.as_bytes())
            .ok_or(Unlowered::Error)?;
        let rvalue_span = self.span(value.span);
        let block = self.block(value, rvalue_span, false, true, input_type)?;
        let output_type = match (block.pipelines.as_slice(), input_type) {
            ([pipeline], Some(input)) if !mutable => {
                check_pipeline_type(self.working_set, pipeline, input)
                    .map_err(|_| Unlowered::Error)?
            }
            _ => block.output_type(),
        };
        let block_id = self.working_set.add_block(Arc::new(block));
        let rvalue = self.node(Expr::Block(block_id), rvalue_span, output_type);
        let (lvalue, explicit_type) = self.variable_declaration(
            &binding.name,
            binding.ty.as_ref(),
            mutable,
            if mutable { None } else { input_type },
        )?;
        let rhs_type = rvalue.ty.clone();
        if let Some(explicit_type) = &explicit_type
            && !type_compatible(explicit_type, &rhs_type)
        {
            return Err(Unlowered::Error);
        }
        if let (Some(var_id), None) = (lvalue.as_var(), &explicit_type) {
            self.working_set.set_variable_type(var_id, rhs_type);
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
            let (var_decl, _) =
                this.variable_declaration(&for_loop.var, for_loop.ty.as_ref(), false, None)?;
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
        let var_type = match iterable_ty {
            Type::OneOf(types) => Type::one_of(types.into_iter().map(yielded_type)),
            ty => yielded_type(ty),
        };
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
    ///    it.
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
        if def.body_params.is_some() {
            return Err(Unlowered::Unsupported("def body with parameters"));
        }
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
            let name = this.node(
                Expr::String(def.name.item.to_string()),
                name_span,
                Type::String,
            );
            let signature = this.signature_expression(&def.signature, false)?;
            let input_type = match &signature.expr {
                Expr::Signature(sig) => sig.get_input_type(),
                _ => Type::Any,
            };
            let body_span = this.braces_span(&def.body)?;
            // `finish_def` compiles the body (`parse_def` marks it the same way). The outer
            // mark is restored before `?`: this `def` may be in another one's body, and a
            // statement handed back does not end the lowering of the statements after it.
            let def_body_span = Some(this.span(body_span));
            let outer_def_body_span =
                std::mem::replace(&mut this.working_set.def_body_span, def_body_span);
            let body = this.closure(
                body_span,
                None,
                &def.body,
                &SyntaxShape::Closure(None),
                Some(&input_type),
            );
            this.working_set.def_body_span = outer_def_body_span;
            let body = body?;
            // The arguments in source order: flags, then the name, the signature and the body.
            let mut arguments: Vec<(usize, Argument)> = def
                .flags
                .iter()
                .map(|flag| {
                    let span = this.span(flag.span);
                    let long = match flag.item {
                        w::DefFlag::Env => "env",
                        w::DefFlag::Wrapped => "wrapped",
                    };
                    let name = Spanned {
                        item: long.to_string(),
                        span,
                    };
                    (span.start, Argument::Named((name, None, None)))
                })
                .collect();
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

    /// `extern name [signature]` (`parse_extern`), with its `attributes`; `command` is the
    /// `extern` expression or the `export` expression around it. As for `def`, the call (the
    /// name and the signature, whose parameters get no variables) is lowered in a scope of its
    /// own; once the scope is left, `finish_extern` makes the predeclared command a known
    /// external.
    pub(super) fn extern_def(
        &mut self,
        pipeline: &w::Pipeline<'s>,
        command: &w::Expression<'s>,
        extern_def: &w::Extern<'s>,
        attributes: &[w::Attribute<'s>],
        module_name: Option<&[u8]>,
    ) -> Lowered<Expression> {
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
            let name = this.node(
                Expr::String(extern_def.name.item.to_string()),
                name_span,
                Type::String,
            );
            call.add_positional(name);
            let signature = this.signature_expression(&extern_def.signature, true)?;
            call.add_positional(signature);
            this.finish_keyword_call(&call, decl_id, None)?;
            Ok(call)
        })?;
        let expression = finish_extern(
            self.working_set,
            Box::new(call),
            call_span,
            name_span,
            desc,
            extra_desc,
            attribute_values,
            module_name,
        );
        self.definition_expression(
            expression,
            lowered_attributes,
            command,
            attributes,
            module_name,
            "export extern",
        )
    }

    /// `const name = value` (`parse_const`): the value is evaluated now and the variable holds
    /// it. In order: the value, a pipeline without input lowered as an `Expr::Subexpression`
    /// (where `let` makes an `Expr::Block`); the variable, declared after it; a written type,
    /// checked against the value's type and then the constant's, and a string held by a `glob`
    /// variable made a glob. A value that does not evaluate is the classic parser's error.
    /// `keyword` is the `const` expression. Returns the statement's expression and the
    /// variable's name span.
    pub(super) fn const_statement(
        &mut self,
        keyword: &w::Expression<'s>,
        binding: &w::Binding<'s>,
    ) -> Lowered<(Expression, Span)> {
        let Some(value) = &binding.value else {
            return Err(Unlowered::Unsupported("const without a value"));
        };
        let decl_id = self
            .working_set
            .find_decl(b"const")
            .ok_or(Unlowered::Error)?;
        let rvalue_span = self.span(value.span);
        let block = self.block(value, rvalue_span, false, true, None)?;
        let rvalue_ty = block.output_type();
        let block_id = self.working_set.add_block(Arc::new(block));
        let rvalue = self.node(Expr::Subexpression(block_id), rvalue_span, rvalue_ty);
        let (lvalue, explicit_type) =
            self.variable_declaration(&binding.name, binding.ty.as_ref(), false, None)?;
        if let Some(explicit_type) = &explicit_type
            && !type_compatible(explicit_type, &rvalue.ty)
        {
            return Err(Unlowered::Error);
        }
        if let Some(var_id) = lvalue.as_var() {
            if explicit_type.is_none() {
                self.working_set
                    .set_variable_type(var_id, rvalue.ty.clone());
            }
            let mut value =
                eval_constant(self.working_set, &rvalue).map_err(|_| Unlowered::Error)?;
            let mut const_type = value.get_type();
            if let Some(explicit_type) = &explicit_type {
                if !type_compatible(explicit_type, &const_type) {
                    return Err(Unlowered::Error);
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
        let head = self.keyword_span(keyword, "const");
        let lvalue_span = lvalue.span;
        let call = Box::new(Call {
            decl_id,
            head,
            arguments: vec![Argument::Positional(lvalue), Argument::Positional(rvalue)],
            parser_info: HashMap::new(),
        });
        let span = self.span(keyword.span);
        Ok((self.node(Expr::Call(call), span, Type::Any), lvalue_span))
    }

    /// `export-env { ... }` (`parse_export_env`): its block is compiled now, once the call is
    /// made. The block is lowered in an extra scope when `export-env`'s signature creates one,
    /// as `parse_internal_call` does for any command. Returns the statement's expression and the
    /// block.
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
        let comments: Vec<Span> = comments.into_iter().map(|span| self.span(span)).collect();
        self.working_set.build_desc(&comments)
    }

    /// Whether only spaces or tabs come before `offset` on its line.
    fn starts_line(&self, offset: usize) -> bool {
        self.source[..offset]
            .rsplit('\n')
            .next()
            .is_none_or(|line| line.trim_start_matches([' ', '\t', '\r']).is_empty())
    }

    /// A definition's attributes and their constant values (the loop that starts `parse_def`
    /// and `parse_extern`). An attribute that does not evaluate to a constant is the classic
    /// parser's error.
    fn attributes(&mut self, attributes: &[w::Attribute<'s>]) -> Lowered<LoweredAttributes> {
        let mut lowered = Vec::with_capacity(attributes.len());
        let mut values = Vec::with_capacity(attributes.len());
        for attribute in attributes {
            let (attribute, name) = self.attribute(attribute)?;
            let value =
                eval_constant(self.working_set, &attribute.expr).map_err(|_| Unlowered::Error)?;
            values.push((name, value));
            lowered.push(attribute);
        }
        Ok((lowered, values))
    }

    /// The head of a definition's call: `keyword`, or `export keyword` when `command` is an
    /// `export`, covering the blanks between the two words, as `Span::concat` of the two items
    /// does.
    fn command_head(&self, command: &w::Expression<'s>, keyword: &str) -> Span {
        let start = self.span(command.span).start;
        match command.expr {
            w::Expr::Export(_) => {
                let after_export = &self.source[command.span.start + "export".len()..];
                let skip = after_export.len() - after_export.trim_start().len();
                Span::new(start, start + "export".len() + skip + keyword.len())
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
        if matches!(command.expr, w::Expr::Export(_)) {
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
            self.export_call(&mut expression, export_name, span)?;
        }
        Ok(expression)
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
    /// covering the attribute without its `@`. Returns it with its name. An alias of an
    /// external command, which is not a constant command, is the classic parser's error.
    fn attribute(&mut self, attribute: &w::Attribute<'s>) -> Lowered<(Attribute, String)> {
        let name = attribute.name.item.to_string();
        let decl_id = self
            .working_set
            .find_decl(format!("attr {name}").as_bytes())
            .ok_or(Unlowered::Error)?;
        if let Some(alias) = self.working_set.get_decl(decl_id).as_alias()
            && matches!(alias.wrapped_call.expr, Expr::ExternalCall(..))
        {
            return Err(Unlowered::Error);
        }
        let name_span = self.span(attribute.name.span);
        let name_span = if self
            .working_set
            .get_span_contents(name_span)
            .starts_with(b"@")
        {
            Span::new(name_span.start + 1, name_span.end)
        } else {
            name_span
        };
        let span = self.span(attribute.span);
        let span = Span::new(span.start + 1, span.end);
        let (call, output) = self.internal_call(name_span, &attribute.arguments, decl_id, None)?;
        let expr = self.node(Expr::Call(call), span, output);
        Ok((Attribute { expr }, name))
    }
}

/// A definition's expression, and the name and declaration of the command it defined.
pub(super) type Definition = (Expression, Option<(Vec<u8>, DeclId)>);

/// A definition's attributes, and their names with their constant values.
type LoweredAttributes = (Vec<Attribute>, Vec<(String, Value)>);

/// What iterating over a value of type `ty` yields (the `yielded_type` inside `parse_for`):
/// recursive, since each alternative of a union may itself be iterable.
fn yielded_type(ty: Type) -> Type {
    match ty {
        Type::List(item) => *item,
        Type::Table(columns) => Type::Record(columns),
        Type::Range => Type::Number,
        Type::OneOf(types) => Type::one_of(types.into_iter().map(yielded_type)),
        ty => ty,
    }
}
