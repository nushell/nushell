//! Keyword statements: `if`, `match`, `while`, `loop`, `try`, `return`, `break`, `continue`
//! (ordinary calls to keyword commands in the classic parser), and `let`, `mut`, `for` and
//! `def` (the classic parser's `parse_let`, `parse_mut`, `parse_for` and `parse_def`).

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
    /// The call to keyword `name` that `e` is, its head the keyword.
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
    /// the output type the keyword's signature gives for `input_type`.
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

    /// A block argument of a keyword (`{ ... }`), or the value standing in for one.
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

    /// The span of a block's braces, from the span of its inside.
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
    /// its branches' outputs (with `nothing` when there is no `else`).
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
        self.finish_keyword_call(&call, decl_id, input_type)?;
        let span = self.span(e.span);
        Ok(self.node(Expr::Call(Box::new(call)), span, output))
    }

    /// The type a branch of `if` or `match` gives: its block's output, or the input for an
    /// empty block.
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

    /// `while condition { body }`.
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

    /// `try { body } catch {|err| ... } finally { ... }`. A `break` in a handler's closure is
    /// allowed although the closure is compiled on its own (the classic parser drops that
    /// error for both of `try`'s handler parameters, whose shapes list `catch`).
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

    /// `match value { pattern => result, ... }`: the output is the union of the arms' outputs
    /// (with `nothing` when no arm matches everything).
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
        self.finish_keyword_call(&call, decl_id, input_type)?;
        let span = self.span(e.span);
        Ok(self.node(Expr::Call(Box::new(call)), span, output))
    }

    /// A match arm's pattern, read by the classic pattern parser (`parse_pattern`), which also
    /// declares its variables.
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

    /// The result of a match arm: a block, or an expression.
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

    /// The variable of `let`, `mut` or `for`, with its type when it has one
    /// (`parse_var_with_opt_type`). An untyped `let` variable takes the pipeline's type.
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
    /// declared; an untyped variable takes the value's type.
    pub(super) fn let_statement(
        &mut self,
        element: &w::PipelineElement<'s>,
        binding: &w::Binding<'s>,
        input_type: Option<&Type>,
    ) -> Lowered<Pipeline> {
        self.binding(element, binding, "let", false, input_type)
    }

    /// `mut name = value` (`parse_mut`).
    pub(super) fn mut_statement(
        &mut self,
        element: &w::PipelineElement<'s>,
        binding: &w::Binding<'s>,
    ) -> Lowered<Pipeline> {
        self.binding(element, binding, "mut", true, None)
    }

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
    /// gets the type of what the iterable yields, and the body's block takes it.
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
    /// command's name and declaration.
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
        let (desc, extra_desc) = self.description(pipeline);
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
            let body = this.closure(
                body_span,
                None,
                &def.body,
                &SyntaxShape::Closure(None),
                Some(&input_type),
            )?;
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
        // reports stands: the statement is not handed back after it.
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
    /// `extern` expression or the `export` expression around it. The predeclared command becomes
    /// a known external.
    pub(super) fn extern_def(
        &mut self,
        pipeline: &w::Pipeline<'s>,
        command: &w::Expression<'s>,
        extern_def: &w::Extern<'s>,
        attributes: &[w::Attribute<'s>],
        module_name: Option<&[u8]>,
    ) -> Lowered<Expression> {
        let (desc, extra_desc) = self.description(pipeline);
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
    /// it. Returns the statement's expression, covering `span`, and the variable's name span.
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

    /// `export-env { ... }` (`parse_export_env`): its block is compiled now. Returns the
    /// statement's expression and the block.
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
    /// after it on its line (`build_desc` over the lite command's comments).
    fn description(&mut self, pipeline: &w::Pipeline<'s>) -> (String, String) {
        let comments: Vec<Span> = pipeline
            .leading_comments
            .iter()
            .chain(&pipeline.trailing_comments)
            .map(|comment| self.span(comment.span))
            .collect();
        self.working_set.build_desc(&comments)
    }

    /// A definition's attributes and their constant values.
    fn attributes(
        &mut self,
        attributes: &[w::Attribute<'s>],
    ) -> Lowered<LoweredAttributes> {
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
    /// `export`.
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
    /// covering the attribute without its `@`. Returns it with its name.
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

/// What iterating over a value of type `ty` yields.
fn yielded_type(ty: Type) -> Type {
    match ty {
        Type::List(item) => *item,
        Type::Table(columns) => Type::Record(columns),
        Type::Range => Type::Number,
        Type::OneOf(types) => Type::one_of(types.into_iter().map(yielded_type)),
        ty => ty,
    }
}
