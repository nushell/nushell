//! Values: literals, strings, variables and cell paths, lists, tables, records, closures and
//! blocks (the classic parser's `parse_value` and what it dispatches to).

use std::sync::Arc;

use nu_protocol::{
    ENV_VARIABLE_ID, IN_VARIABLE_ID, LAST_RESULT_VAR_NAME, LAST_VARIABLE_ID, NU_VARIABLE_ID,
    PositionalArg, Signature, Span, SyntaxShape, Type, TypeSet,
    ast::{CellPath, Expr, Expression, FullCellPath, ListItem, PathMember, RecordItem, Table},
    casing::Casing,
};
use nu_winnow_parser::{Span as WSpan, ast as w};

use super::{Lower, Lowered, Unlowered};
use crate::{
    parse_captures_compile::compile_block,
    parse_expressions::parse_value as classic_value,
    parse_expressions::{check_record_key_or_value, table_type},
    parse_literals::is_quoted,
};

impl<'s> Lower<'_, '_, 's> {
    /// One item read with `shape` (`parse_value`). As there, the item's first byte picks how it
    /// is read, and the shape matters within each case.
    pub(super) fn value(
        &mut self,
        e: &w::Expression<'s>,
        shape: &SyntaxShape,
        input_type: Option<&Type>,
    ) -> Lowered<Expression> {
        if let SyntaxShape::OneOf(shapes) = shape {
            return self.one_of(e, shapes, input_type);
        }
        match self.text(e.span).as_bytes() {
            [] => Err(Unlowered::Error),
            [b'$', ..] => self.dollar(e, shape, input_type),
            [b'(', ..] => self.paren(e, shape),
            [b'{', ..] => self.brace(e, shape, input_type),
            [b'[', ..] => self.bracket(e, shape),
            _ => self.leaf(e, shape),
        }
    }

    /// The first of `shapes` that reads the item without an error (`parse_oneof`). When none
    /// does, the classic parser picks the error to report. A shape that is handed back, or that
    /// lowers but leaves errors (from a statement of a nested block the classic parser parsed),
    /// does not end the search: its errors are dropped and the next shape is tried.
    pub(super) fn one_of(
        &mut self,
        e: &w::Expression<'s>,
        shapes: &[SyntaxShape],
        input_type: Option<&Type>,
    ) -> Lowered<Expression> {
        for shape in shapes {
            let before = self.working_set.parse_errors.len();
            match self.value(e, shape, input_type) {
                Ok(value) if self.working_set.parse_errors.len() == before => return Ok(value),
                _ => self.working_set.parse_errors.truncate(before),
            }
        }
        Err(Unlowered::Error)
    }

    /// An item without inner structure: a number, a word, a quoted string. The common shapes
    /// are read from the winnow literal; the others (a bare word as a path, a glob, a cell
    /// path, a number as a string, units, dates, ranges) by the classic leaf parsers, whose
    /// errors hand the statement back.
    fn leaf(&mut self, e: &w::Expression<'s>, shape: &SyntaxShape) -> Lowered<Expression> {
        let span = self.span(e.span);
        let (expr, ty) = match (shape, &e.expr) {
            (SyntaxShape::Any | SyntaxShape::Int | SyntaxShape::Number, w::Expr::Int(value)) => {
                (Expr::Int(*value), Type::Int)
            }
            (
                SyntaxShape::Any | SyntaxShape::Float | SyntaxShape::Number,
                w::Expr::Float(value),
            ) => (Expr::Float(*value), Type::Float),
            (SyntaxShape::Any | SyntaxShape::Boolean, w::Expr::Bool(value)) => {
                (Expr::Bool(*value), Type::Bool)
            }
            (SyntaxShape::Any | SyntaxShape::Nothing, w::Expr::Nothing) => {
                (Expr::Nothing, Type::Nothing)
            }
            (SyntaxShape::Any | SyntaxShape::String, w::Expr::String(string)) => match string.quote
            {
                w::Quote::Raw(_) => (Expr::RawString(string.value.to_string()), Type::String),
                _ => (Expr::String(string.value.to_string()), Type::String),
            },
            (
                SyntaxShape::Any
                | SyntaxShape::String
                | SyntaxShape::Filepath
                | SyntaxShape::Directory
                | SyntaxShape::GlobPattern,
                w::Expr::StringInterpolation(interpolation),
            ) => return self.path_interpolation(e.span, interpolation, shape),
            // A raw string is one whatever the shape (`parse_value` reads `r#` first).
            (_, w::Expr::String(string)) if matches!(string.quote, w::Quote::Raw(_)) => {
                (Expr::RawString(string.value.to_string()), Type::String)
            }
            // A word or quoted string as a path, a directory or a glob (`parse_path_like`),
            // quoted when it has quotes (backticks do not count).
            (
                SyntaxShape::Filepath | SyntaxShape::Directory | SyntaxShape::GlobPattern,
                w::Expr::String(string),
            ) => {
                let quoted = matches!(string.quote, w::Quote::Single | w::Quote::Double);
                let value = string.value.to_string();
                match shape {
                    SyntaxShape::Filepath => (Expr::Filepath(value, quoted), Type::String),
                    SyntaxShape::Directory => (Expr::Directory(value, quoted), Type::String),
                    _ => (Expr::GlobPattern(value, quoted), Type::Glob),
                }
            }
            _ => {
                return self.checked(|working_set| classic_value(working_set, span, shape, None));
            }
        };
        Ok(self.node(expr, span, ty))
    }

    /// An item starting with `$` (`parse_dollar_expr`).
    fn dollar(
        &mut self,
        e: &w::Expression<'s>,
        shape: &SyntaxShape,
        input_type: Option<&Type>,
    ) -> Lowered<Expression> {
        match &e.expr {
            w::Expr::StringInterpolation(interpolation) => {
                self.path_interpolation(e.span, interpolation, shape)
            }
            w::Expr::CellPath(path) => {
                // `$.a`: the literal starts after the `$.`.
                let span = self.span(e.span);
                let span = Span::new(span.start + 2, span.end);
                let members = self.members(&path.members);
                Ok(self.node(Expr::CellPath(CellPath { members }), span, Type::CellPath))
            }
            w::Expr::Range(_) => self.leaf(e, shape),
            w::Expr::Var(_) => self.full_cell_path(e.span, e, &[], input_type),
            w::Expr::FullCellPath(path) if !path.implicit_head => {
                self.full_cell_path(e.span, &path.head, &path.tail, input_type)
            }
            _ => Err(Unlowered::Unsupported("`$` item")),
        }
    }

    /// An item starting with `(` (`parse_paren_expr`).
    fn paren(&mut self, e: &w::Expression<'s>, shape: &SyntaxShape) -> Lowered<Expression> {
        match &e.expr {
            w::Expr::Range(_) => self.leaf(e, shape),
            _ if matches!(
                shape,
                SyntaxShape::Signature | SyntaxShape::ExternalSignature
            ) =>
            {
                Err(Unlowered::Unsupported("parenthesized signature"))
            }
            w::Expr::Subexpression(_) => self.full_cell_path(e.span, e, &[], None),
            w::Expr::FullCellPath(path) if !path.implicit_head => {
                self.full_cell_path(e.span, &path.head, &path.tail, None)
            }
            // `(pwd)/x`: a bare interpolation.
            w::Expr::StringInterpolation(interpolation) => {
                self.path_interpolation(e.span, interpolation, shape)
            }
            _ => Err(Unlowered::Unsupported("`(` item")),
        }
    }

    /// An item starting with `{` (`parse_brace_expr`): a record, a closure or a block.
    fn brace(
        &mut self,
        e: &w::Expression<'s>,
        shape: &SyntaxShape,
        input_type: Option<&Type>,
    ) -> Lowered<Expression> {
        match &e.expr {
            // `{}`: what the shape asks for.
            w::Expr::Record(items) if items.is_empty() => match shape {
                SyntaxShape::Closure(_) => {
                    let body = w::Block::default();
                    self.closure(e.span, None, &body, shape, None)
                }
                SyntaxShape::Block => {
                    self.block_expression(e.span, &w::Block::default(), input_type)
                }
                SyntaxShape::MatchBlock => Err(Unlowered::Unsupported("empty match block")),
                _ => self.record(e.span, items),
            },
            // A record starting with a spread is read as a record whatever the shape, unless the
            // shape wants a closure or a block, which the lowering leaves to the classic parser.
            w::Expr::Record(items)
                if matches!(items.first(), Some(w::RecordItem::Spread { .. })) =>
            {
                match shape {
                    SyntaxShape::Closure(_) | SyntaxShape::Block | SyntaxShape::MatchBlock => {
                        Err(Unlowered::Unsupported("spread record as a block"))
                    }
                    _ => self.record(e.span, items),
                }
            }
            // `{a: 1}` is a record (with a cell path) whatever the shape.
            w::Expr::Record(_) => self.full_cell_path(e.span, e, &[], None),
            w::Expr::FullCellPath(path) if matches!(path.head.expr, w::Expr::Record(_)) => {
                self.full_cell_path(e.span, &path.head, &path.tail, None)
            }
            // As `parse_brace_expr` decides: with parameters, a closure for any shape but a
            // block (an error); without, a block for a block shape, a closure for a closure or
            // any shape, an error for the rest. A match block is left to the classic parser.
            w::Expr::Closure(closure) => {
                let has_params = closure.params.is_some();
                match shape {
                    SyntaxShape::Block if has_params => Err(Unlowered::Error),
                    SyntaxShape::Block => self.block_expression(e.span, &closure.body, input_type),
                    SyntaxShape::MatchBlock => Err(Unlowered::Unsupported("match block value")),
                    SyntaxShape::Closure(_) | SyntaxShape::Any => {
                        self.closure(e.span, closure.params.as_ref(), &closure.body, shape, None)
                    }
                    _ if has_params => {
                        self.closure(e.span, closure.params.as_ref(), &closure.body, shape, None)
                    }
                    _ => Err(Unlowered::Error),
                }
            }
            _ => Err(Unlowered::Unsupported("`{` item")),
        }
    }

    /// An item starting with `[` (the `[` case of `parse_value`): a list or a table, possibly
    /// with a cell path. A shape that does not take a `[` item is the classic parser's error.
    fn bracket(&mut self, e: &w::Expression<'s>, shape: &SyntaxShape) -> Lowered<Expression> {
        match shape {
            SyntaxShape::List(element) => self.table_expression(e, element),
            SyntaxShape::Table(_) => self.table_expression(e, &SyntaxShape::Any),
            SyntaxShape::Any => match &e.expr {
                w::Expr::FullCellPath(path) if !path.implicit_head => {
                    self.full_cell_path(e.span, &path.head, &path.tail, None)
                }
                _ => self.full_cell_path(e.span, e, &[], None),
            },
            SyntaxShape::Signature | SyntaxShape::ExternalSignature => {
                Err(Unlowered::Unsupported("signature value"))
            }
            SyntaxShape::Filepath
            | SyntaxShape::String
            | SyntaxShape::GlobPattern
            | SyntaxShape::ExternalArgument => self.leaf(e, shape),
            _ => Err(Unlowered::Error),
        }
    }

    /// A head with an optional cell path after it (`parse_full_cell_path`), covering `span`.
    /// Its type is the head's or, with members, what following them through the head's type
    /// gives when `nu_experimental::CELL_PATH_TYPES` is on (`any` otherwise).
    pub(super) fn full_cell_path(
        &mut self,
        span: WSpan,
        head: &w::Expression<'s>,
        tail: &[w::PathMember<'s>],
        input_type: Option<&Type>,
    ) -> Lowered<Expression> {
        let head = match &head.expr {
            w::Expr::Var(var) => self.variable(head.span, var.name, input_type)?,
            w::Expr::Subexpression(block) => self.subexpression(head.span, block)?,
            w::Expr::List(_) | w::Expr::Table(_) => {
                self.table_expression(head, &SyntaxShape::Any)?
            }
            w::Expr::Record(items) => self.record(head.span, items)?,
            _ => return Err(Unlowered::Unsupported("cell path head")),
        };
        let tail = self.members(tail);
        let ty = if tail.is_empty() {
            head.ty.clone()
        } else if nu_experimental::CELL_PATH_TYPES.get() {
            head.ty
                .follow_cell_path(&tail)
                .map(|ty| ty.into_owned())
                .unwrap_or(Type::Any)
        } else {
            Type::Any
        };
        let span = self.span(span);
        Ok(self.node(
            Expr::FullCellPath(Box::new(FullCellPath { head, tail })),
            span,
            ty,
        ))
    }

    /// A variable (`parse_variable_expr`): `$nu`, `$in` (typed by `input_type`), `$env` and the
    /// last-result variable have fixed ids; any other name must be in scope, else it is the
    /// classic parser's error.
    pub(super) fn variable(
        &mut self,
        span: WSpan,
        name: &str,
        input_type: Option<&Type>,
    ) -> Lowered<Expression> {
        let span = self.span(span);
        let (var_id, ty) = match name {
            "nu" => (NU_VARIABLE_ID, Type::Any),
            "in" => (IN_VARIABLE_ID, input_type.cloned().unwrap_or(Type::Any)),
            "env" => (ENV_VARIABLE_ID, Type::Any),
            _ if name == LAST_RESULT_VAR_NAME => (LAST_VARIABLE_ID, Type::Any),
            _ => {
                let var_id = self
                    .working_set
                    .find_variable(self.working_set.get_span_contents(span))
                    .ok_or(Unlowered::Error)?;
                (var_id, self.working_set.get_variable(var_id).ty.clone())
            }
        };
        Ok(self.node(Expr::Var(var_id), span, ty))
    }

    /// The members of a cell path (`parse_cell_path`). A member's span leaves out its `?` and
    /// `!` modifiers, which the classic lexer reads as tokens of their own.
    pub(super) fn members(&self, members: &[w::PathMember<'s>]) -> Vec<PathMember> {
        members
            .iter()
            .map(|member| {
                let modifiers = usize::from(member.optional) + usize::from(member.case_insensitive);
                let span = self.span(member.span);
                let span = Span::new(span.start, span.end - modifiers);
                let mut path_member = match &member.kind {
                    w::PathMemberKind::Int(value) => PathMember::int(*value, false, span),
                    w::PathMemberKind::String(value) => {
                        PathMember::string(value.to_string(), false, Casing::Sensitive, span)
                    }
                };
                if member.optional {
                    path_member.make_optional();
                }
                if member.case_insensitive {
                    path_member.make_insensitive();
                }
                path_member
            })
            .collect()
    }

    /// A parenthesized subexpression, covering `span` with its parentheses (the head of
    /// `parse_full_cell_path`): its block covers the inside and gets a scope of its own.
    fn subexpression(&mut self, span: WSpan, block: &w::Block<'s>) -> Lowered<Expression> {
        let span = self.span(span);
        let inner = Span::new(span.start + 1, span.end - 1);
        let block = self.block(block, inner, true, true, None)?;
        let ty = block.output_type();
        let block_id = self.working_set.add_block(Arc::new(block));
        Ok(self.node(Expr::Subexpression(block_id), span, ty))
    }

    /// An interpolation read with `shape`: for a glob, a glob interpolation (`parse_path_like`)
    /// when the item has a `(`, quoted only when the whole item is in quotes, so that `$"..."`,
    /// which starts with `$`, is a bare one; a string interpolation otherwise.
    fn path_interpolation(
        &mut self,
        span: WSpan,
        interpolation: &w::StringInterpolation<'s>,
        shape: &SyntaxShape,
    ) -> Lowered<Expression> {
        let expression = self.interpolation(span, interpolation)?;
        match (shape, expression.expr) {
            // The classic `parse_dollar_expr` takes the glob route only for an item with a `(`
            // (`is_bare_string_interpolation`): `$"foo"` stays a string interpolation.
            (SyntaxShape::GlobPattern, Expr::StringInterpolation(parts))
                if self.text(span).contains('(') =>
            {
                let quoted = is_quoted(self.text(span).as_bytes());
                Ok(self.node(
                    Expr::GlobInterpolation(parts, quoted),
                    expression.span,
                    expression.ty,
                ))
            }
            (_, expr) => Ok(Expression { expr, ..expression }),
        }
    }

    /// A string with interpolated expressions (`parse_string_interpolation`): text parts are
    /// strings, and each `(...)` part a subexpression, read as the head of a full cell path as
    /// the classic parser reads it. Any other expression part is left to the classic parser.
    fn interpolation(
        &mut self,
        span: WSpan,
        interpolation: &w::StringInterpolation<'s>,
    ) -> Lowered<Expression> {
        let mut parts = Vec::with_capacity(interpolation.parts.len());
        for part in &interpolation.parts {
            parts.push(match part {
                w::InterpolationPart::Text { span, value } => {
                    let span = self.span(*span);
                    self.node(Expr::String(value.to_string()), span, Type::String)
                }
                w::InterpolationPart::Expression(expression) => match &expression.expr {
                    w::Expr::Subexpression(_) => {
                        self.full_cell_path(expression.span, expression, &[], None)?
                    }
                    _ => return Err(Unlowered::Unsupported("interpolated expression")),
                },
            });
        }
        let span = self.span(span);
        Ok(self.node(Expr::StringInterpolation(parts), span, Type::String))
    }

    /// A list or a table (`parse_table_expression`), its items read with `element`.
    fn table_expression(
        &mut self,
        e: &w::Expression<'s>,
        element: &SyntaxShape,
    ) -> Lowered<Expression> {
        match &e.expr {
            w::Expr::List(items) => self.list(e.span, items, element),
            w::Expr::Table(table) => self.table(e.span, table),
            _ => Err(Unlowered::Unsupported("list item")),
        }
    }

    /// A list (`parse_list_expression`); its type is the union of its items' types. A spread,
    /// `...$items`, is read as a list of `element` and adds its item type.
    pub(super) fn list(
        &mut self,
        span: WSpan,
        items: &[w::ListItem<'s>],
        element: &SyntaxShape,
    ) -> Lowered<Expression> {
        let mut out = Vec::with_capacity(items.len());
        let mut contained: Option<Type> = None;
        for item in items {
            let (item, ty) = match item {
                w::ListItem::Item(expression) => {
                    let value = self.value(expression, element, None)?;
                    let ty = value.ty.clone();
                    (ListItem::Item(value), ty)
                }
                w::ListItem::Spread { dots, expr } => {
                    let list_shape = SyntaxShape::List(Box::new(element.clone()));
                    let value = self.value(expr, &list_shape, None)?;
                    let ty = match &value.ty {
                        Type::List(ty) => (**ty).clone(),
                        _ => Type::Any,
                    };
                    (ListItem::Spread(self.span(*dots), value), ty)
                }
            };
            contained = Some(match contained {
                Some(contained) => contained.union(ty),
                None => ty,
            });
            out.push(item);
        }
        let span = self.span(span);
        let ty = Type::List(Box::new(contained.unwrap_or(Type::Any)));
        Ok(self.node(Expr::List(out), span, ty))
    }

    /// A table literal: a header row, `;`, then rows (`parse_table_expression`), typed by
    /// `table_type`. A row of another length than the header, a header without rows, and
    /// column types `table_type` rejects are the classic parser's errors.
    fn table(&mut self, span: WSpan, table: &w::Table<'s>) -> Lowered<Expression> {
        let columns = self.table_row(&table.columns)?;
        let mut rows = Vec::with_capacity(table.rows.len());
        for row in &table.rows {
            let row = self.table_row(row)?;
            if row.len() != columns.len() {
                return Err(Unlowered::Error);
            }
            rows.push(row);
        }
        if rows.is_empty() {
            return Err(Unlowered::Error);
        }
        let (ty, errors) = table_type(&columns, &rows);
        if !errors.is_empty() {
            return Err(Unlowered::Error);
        }
        let span = self.span(span);
        let table = Table {
            columns: columns.into(),
            rows: rows.into_iter().map(Into::into).collect(),
        };
        Ok(self.node(Expr::Table(table), span, ty))
    }

    /// One row of a table, whose items cannot be spreads (`parse_table_row`).
    fn table_row(&mut self, row: &w::Expression<'s>) -> Lowered<Vec<Expression>> {
        let w::Expr::List(items) = &row.expr else {
            return Err(Unlowered::Error);
        };
        let mut out = Vec::with_capacity(items.len());
        for item in items {
            match item {
                w::ListItem::Item(expression) => {
                    out.push(self.value(expression, &SyntaxShape::Any, None)?)
                }
                w::ListItem::Spread { .. } => return Err(Unlowered::Error),
            }
        }
        Ok(out)
    }

    /// A record (`parse_record`). Its type lists the field types when every key is a literal
    /// string and every spread a record of known fields; otherwise it is `any`. A key or value
    /// that is a bare word with a `:` is the classic parser's error (`check_record_key_or_value`).
    fn record(&mut self, span: WSpan, items: &[w::RecordItem<'s>]) -> Lowered<Expression> {
        let mut out = Vec::with_capacity(items.len());
        let mut field_types = Some(Vec::new());
        for item in items {
            match item {
                w::RecordItem::Spread { dots, expr } => {
                    let inner = self.value(expr, &SyntaxShape::record(), None)?;
                    match &inner.ty {
                        Type::Record(fields) => {
                            if let Some(field_types) = &mut field_types {
                                field_types.extend(fields.iter().cloned());
                            }
                        }
                        _ => field_types = None,
                    }
                    out.push(RecordItem::Spread(self.span(*dots), inner));
                }
                w::RecordItem::Pair { key, value, .. } => {
                    let key = self.value(key, &SyntaxShape::String, None)?;
                    if check_record_key_or_value(self.working_set, &key, "key").is_some() {
                        return Err(Unlowered::Error);
                    }
                    let value = self.value(value, &SyntaxShape::Any, None)?;
                    if check_record_key_or_value(self.working_set, &value, "value").is_some() {
                        return Err(Unlowered::Error);
                    }
                    match key.as_string() {
                        Some(field) => {
                            if let Some(field_types) = &mut field_types {
                                field_types.push((field, value.ty.clone()));
                            }
                        }
                        None => field_types = None,
                    }
                    out.push(RecordItem::Pair(key, value));
                }
            }
        }
        let span = self.span(span);
        let ty = match field_types {
            Some(fields) => Type::Record(fields.into()),
            None => Type::Any,
        };
        Ok(self.node(Expr::Record(out), span, ty))
    }

    /// A closure (`parse_closure_expression`), covering `span` with its braces. In its own
    /// scope, as the classic parser makes it:
    ///
    /// 1. The parameters, declared in that scope and checked against those a `closure(...)`
    ///    shape expects.
    /// 2. The body's block, receiving `input_type`.
    /// 3. The block compiled right away, as every closure is, except when the working set holds
    ///    any parse error (the classic parser's check, over everything parsed so far) and for
    ///    the body of the `def` being lowered, which `finish_def` compiles.
    /// 4. The parameters made the block's signature, and the scope's bindings recorded.
    pub(super) fn closure(
        &mut self,
        span: WSpan,
        params: Option<&w::Signature<'s>>,
        body: &w::Block<'s>,
        shape: &SyntaxShape,
        input_type: Option<&Type>,
    ) -> Lowered<Expression> {
        let span = self.span(span);
        let block = self.in_scope(|this| {
            let signature = match params {
                Some(params) => Some(this.closure_signature(params)?),
                None => None,
            };
            if let (SyntaxShape::Closure(Some(expected)), Some(signature)) = (shape, &signature) {
                if signature.num_positionals() > expected.len() {
                    return Err(Unlowered::Error);
                }
                for (expected, PositionalArg { shape, .. }) in
                    expected.iter().zip(signature.required_positional.iter())
                {
                    if expected != shape && *shape != SyntaxShape::Any {
                        return Err(Unlowered::Error);
                    }
                }
            }
            let mut block = this.block(body, span, false, false, input_type)?;
            // A `def` body is compiled by `finish_def`, once its scope is closed.
            if this.working_set.parse_errors.is_empty()
                && this.working_set.def_body_span != Some(span)
            {
                compile_block(this.working_set, &mut block);
            }
            if let Some(signature) = signature {
                block.signature = signature;
            }
            block.span = Some(span);
            block.scope_bindings = this.working_set.snapshot_scope_bindings();
            Ok(block)
        })?;
        let block_id = self.working_set.add_block(Arc::new(block));
        Ok(self.node(Expr::Closure(block_id), span, Type::Closure))
    }

    /// A block in braces (`parse_block_expression`), covering `span`, in its own scope; not
    /// compiled on its own but with the block it is in.
    pub(super) fn block_expression(
        &mut self,
        span: WSpan,
        body: &w::Block<'s>,
        input_type: Option<&Type>,
    ) -> Lowered<Expression> {
        let span = self.span(span);
        let block = self.in_scope(|this| {
            let mut block = this.block(body, span, false, false, input_type)?;
            block.span = Some(span);
            block.scope_bindings = this.working_set.snapshot_scope_bindings();
            Ok(block)
        })?;
        let block_id = self.working_set.add_block(Arc::new(block));
        Ok(self.node(Expr::Block(block_id), span, Type::Block))
    }

    /// The parameters of a closure, `|a, b: int|`: an unnamed signature, or one named
    /// `closure` for `||`, as the classic parser makes them.
    fn closure_signature(&mut self, params: &w::Signature<'s>) -> Lowered<Box<Signature>> {
        if self.text(params.span) == "||" {
            return Ok(Box::new(Signature::new("closure".to_string())));
        }
        self.signature_params(params, false)
    }
}
