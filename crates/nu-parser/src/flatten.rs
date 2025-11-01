use nu_protocol::{
    DeclId, GetSpan, Span, SyntaxShape, VarId,
    ast::{
        Argument, Block, Expr, Expression, ExternalArgument, ImportPatternMember, ListItem,
        MatchPattern, PathMember, Pattern, Pipeline, PipelineElement, PipelineRedirection,
        RecordItem,
    },
    engine::StateWorkingSet,
};
use std::fmt::{Display, Formatter, Result};

#[derive(Debug, Eq, PartialEq, Ord, Clone, PartialOrd)]
pub enum FlatShape {
    Binary,
    Block,
    Bool,
    Closure,
    Custom(DeclId),
    DateTime,
    Directory,
    // The stored span contains the call's head if this call is through an alias:
    // This is only different from the name of the called external command,
    // and is only useful for its location (not its contents).
    External(Box<Span>),
    ExternalArg,
    ExternalResolved,
    Filepath,
    Flag,
    Float,
    Garbage,
    GlobInterpolation,
    GlobPattern,
    Int,
    InternalCall(DeclId),
    Keyword,
    List,
    Literal,
    MatchPattern,
    Nothing,
    Operator,
    Pipe,
    Range,
    RawString,
    Record,
    Redirection,
    Signature,
    String,
    StringInterpolation,
    Table,
    Variable(VarId),
    VarDecl(VarId),
}

impl FlatShape {
    pub fn as_str(&self) -> &str {
        match self {
            FlatShape::Binary => "shape_binary",
            FlatShape::Block => "shape_block",
            FlatShape::Bool => "shape_bool",
            FlatShape::Closure => "shape_closure",
            FlatShape::Custom(_) => "shape_custom",
            FlatShape::DateTime => "shape_datetime",
            FlatShape::Directory => "shape_directory",
            FlatShape::External(_) => "shape_external",
            FlatShape::ExternalArg => "shape_externalarg",
            FlatShape::ExternalResolved => "shape_external_resolved",
            FlatShape::Filepath => "shape_filepath",
            FlatShape::Flag => "shape_flag",
            FlatShape::Float => "shape_float",
            FlatShape::Garbage => "shape_garbage",
            FlatShape::GlobInterpolation => "shape_glob_interpolation",
            FlatShape::GlobPattern => "shape_globpattern",
            FlatShape::Int => "shape_int",
            FlatShape::InternalCall(_) => "shape_internalcall",
            FlatShape::Keyword => "shape_keyword",
            FlatShape::List => "shape_list",
            FlatShape::Literal => "shape_literal",
            FlatShape::MatchPattern => "shape_match_pattern",
            FlatShape::Nothing => "shape_nothing",
            FlatShape::Operator => "shape_operator",
            FlatShape::Pipe => "shape_pipe",
            FlatShape::Range => "shape_range",
            FlatShape::RawString => "shape_raw_string",
            FlatShape::Record => "shape_record",
            FlatShape::Redirection => "shape_redirection",
            FlatShape::Signature => "shape_signature",
            FlatShape::String => "shape_string",
            FlatShape::StringInterpolation => "shape_string_interpolation",
            FlatShape::Table => "shape_table",
            FlatShape::Variable(_) => "shape_variable",
            FlatShape::VarDecl(_) => "shape_vardecl",
        }
    }
}

impl Display for FlatShape {
    fn fmt(&self, f: &mut Formatter) -> Result {
        f.write_str(self.as_str())
    }
}

/*
The `_into` functions below (e.g., `flatten_block_into`) take an existing `output` `Vec`
and append more data to it. This is to reduce the number of intermediate `Vec`s.
The non-`into` functions (e.g., `flatten_block`) are part of the crate's public API
and return a new `Vec` instead of modifying an existing one.
*/

fn flatten_block_into(
    working_set: &StateWorkingSet,
    block: &Block,
    output: &mut Vec<(Span, FlatShape)>,
) {
    for pipeline in &block.pipelines {
        flatten_pipeline_into(working_set, pipeline, output);
    }
}

fn flatten_pipeline_into(
    working_set: &StateWorkingSet,
    pipeline: &Pipeline,
    output: &mut Vec<(Span, FlatShape)>,
) {
    for expr in &pipeline.elements {
        flatten_pipeline_element_into(working_set, expr, output)
    }
}

fn flatten_pipeline_element_into(
    working_set: &StateWorkingSet,
    pipeline_element: &PipelineElement,
    output: &mut Vec<(Span, FlatShape)>,
) {
    if let Some(span) = pipeline_element.pipe {
        output.push((span, FlatShape::Pipe));
    }

    flatten_expression_into(working_set, &pipeline_element.expr, output);

    if let Some(redirection) = pipeline_element.redirection.as_ref() {
        match redirection {
            PipelineRedirection::Single { target, .. } => {
                output.push((target.span(), FlatShape::Redirection));
                if let Some(expr) = target.expr() {
                    flatten_expression_into(working_set, expr, output);
                }
            }
            PipelineRedirection::Separate { out, err } => {
                let (out, err) = if out.span() <= err.span() {
                    (out, err)
                } else {
                    (err, out)
                };

                output.push((out.span(), FlatShape::Redirection));
                if let Some(expr) = out.expr() {
                    flatten_expression_into(working_set, expr, output);
                }
                output.push((err.span(), FlatShape::Redirection));
                if let Some(expr) = err.expr() {
                    flatten_expression_into(working_set, expr, output);
                }
            }
        }
    }
}

fn flatten_positional_arg_into(
    working_set: &StateWorkingSet,
    positional: &Expression,
    shape: &SyntaxShape,
    output: &mut Vec<(Span, FlatShape)>,
) {
    if matches!(shape, SyntaxShape::ExternalArgument)
        && matches!(positional.expr, Expr::String(..) | Expr::GlobPattern(..))
    {
        // Make known external arguments look more like external arguments
        output.push((positional.span, FlatShape::ExternalArg));
    } else {
        flatten_expression_into(working_set, positional, output)
    }
}

fn flatten_expression_into(
    working_set: &StateWorkingSet,
    expr: &Expression,
    output: &mut Vec<(Span, FlatShape)>,
) {
    match &expr.expr {
        Expr::AttributeBlock(ab) => {
            for attr in &ab.attributes {
                flatten_expression_into(working_set, &attr.expr, output);
            }
            flatten_expression_into(working_set, &ab.item, output);
        }
        Expr::BinaryOp(lhs, op, rhs) => {
            flatten_expression_into(working_set, lhs, output);
            flatten_expression_into(working_set, op, output);
            flatten_expression_into(working_set, rhs, output);
        }
        Expr::UnaryNot(not) => {
            output.push((
                Span::new(expr.span.start, expr.span.start + 3),
                FlatShape::Operator,
            ));
            flatten_expression_into(working_set, not, output);
        }
        Expr::Collect(_, expr) => {
            flatten_expression_into(working_set, expr, output);
        }
        Expr::Closure(block_id) => {
            let outer_span = expr.span;

            let block = working_set.get_block(*block_id);
            let flattened = flatten_block(working_set, block);

            if let Some(first) = flattened.first()
                && first.0.start > outer_span.start
            {
                output.push((
                    Span::new(outer_span.start, first.0.start),
                    FlatShape::Closure,
                ));
            }

            let last = if let Some(last) = flattened.last() {
                if last.0.end < outer_span.end {
                    Some((Span::new(last.0.end, outer_span.end), FlatShape::Closure))
                } else {
                    None
                }
            } else {
                // for empty closures
                Some((outer_span, FlatShape::Closure))
            };

            output.extend(flattened);
            if let Some(last) = last {
                output.push(last);
            }
        }
        Expr::Block(block_id) | Expr::RowCondition(block_id) | Expr::Subexpression(block_id) => {
            let outer_span = expr.span;

            let flattened = flatten_block(working_set, working_set.get_block(*block_id));

            if let Some(first) = flattened.first()
                && first.0.start > outer_span.start
            {
                output.push((Span::new(outer_span.start, first.0.start), FlatShape::Block));
            }

            let last = if let Some(last) = flattened.last() {
                if last.0.end < outer_span.end {
                    Some((Span::new(last.0.end, outer_span.end), FlatShape::Block))
                } else {
                    None
                }
            } else {
                None
            };

            output.extend(flattened);
            if let Some(last) = last {
                output.push(last);
            }
        }
        Expr::Call(call) => {
            let decl = working_set.get_decl(call.decl_id);

            if call.head.end != 0 {
                // Make sure we don't push synthetic calls
                output.push((call.head, FlatShape::InternalCall(call.decl_id)));
            }

            // Follow positional arguments from the signature.
            let signature = decl.signature();
            let mut positional_args = signature
                .required_positional
                .iter()
                .chain(&signature.optional_positional);

            let arg_start = output.len();
            for arg in &call.arguments {
                match arg {
                    Argument::Positional(positional) => {
                        let positional_arg = positional_args.next();
                        let shape = positional_arg
                            .or(signature.rest_positional.as_ref())
                            .map(|arg| &arg.shape)
                            .unwrap_or(&SyntaxShape::Any);

                        flatten_positional_arg_into(working_set, positional, shape, output)
                    }
                    Argument::Unknown(positional) => {
                        let shape = signature
                            .rest_positional
                            .as_ref()
                            .map(|arg| &arg.shape)
                            .unwrap_or(&SyntaxShape::Any);

                        flatten_positional_arg_into(working_set, positional, shape, output)
                    }
                    Argument::Named(named) => {
                        if named.0.span.end != 0 {
                            // Ignore synthetic flags
                            output.push((named.0.span, FlatShape::Flag));
                        }
                        if let Some(expr) = &named.2 {
                            flatten_expression_into(working_set, expr, output);
                        }
                    }
                    Argument::Spread(expr) => {
                        output.push((
                            Span::new(expr.span.start - 3, expr.span.start),
                            FlatShape::Operator,
                        ));
                        flatten_expression_into(working_set, expr, output);
                    }
                }
            }
            // sort these since flags and positional args can be intermixed
            output[arg_start..].sort();
        }
        Expr::ExternalCall(head, args) => {
            if let Expr::String(..) | Expr::GlobPattern(..) = &head.expr {
                // If this external call is through an alias, then head.span contains the
                // name of the alias (needed to highlight the right thing), but we also need
                // the name of the aliased command (to decide *how* to highlight the call).
                // The parser actually created this head by cloning from the alias's definition
                // and then just overwriting the `span` field - but `span_id` still points to
                // the original span, so we can recover it from there.
                let span = working_set.get_span(head.span_id);
                output.push((span, FlatShape::External(Box::new(head.span))));
            } else {
                flatten_expression_into(working_set, head, output);
            }

            for arg in args.as_ref() {
                match arg {
                    ExternalArgument::Regular(expr) => {
                        if let Expr::String(..) | Expr::GlobPattern(..) = &expr.expr {
                            output.push((expr.span, FlatShape::ExternalArg));
                        } else {
                            flatten_expression_into(working_set, expr, output);
                        }
                    }
                    ExternalArgument::Spread(expr) => {
                        output.push((
                            Span::new(expr.span.start - 3, expr.span.start),
                            FlatShape::Operator,
                        ));
                        flatten_expression_into(working_set, expr, output);
                    }
                }
            }
        }
        Expr::Garbage => output.push((expr.span, FlatShape::Garbage)),
        Expr::Nothing => output.push((expr.span, FlatShape::Nothing)),
        Expr::DateTime(_) => output.push((expr.span, FlatShape::DateTime)),
        Expr::Binary(_) => output.push((expr.span, FlatShape::Binary)),
        Expr::Int(_) => output.push((expr.span, FlatShape::Int)),
        Expr::Float(_) => output.push((expr.span, FlatShape::Float)),
        Expr::MatchBlock(matches) => {
            for (pattern, expr) in matches {
                flatten_pattern_into(pattern, output);
                flatten_expression_into(working_set, expr, output);
            }
        }
        Expr::ValueWithUnit(value) => {
            flatten_expression_into(working_set, &value.expr, output);
            output.push((value.unit.span, FlatShape::String));
        }
        Expr::CellPath(cell_path) => {
            output.extend(cell_path.members.iter().map(|member| match *member {
                PathMember::String { span, .. } => (span, FlatShape::String),
                PathMember::Int { span, .. } => (span, FlatShape::Int),
            }));
        }
        Expr::FullCellPath(cell_path) => {
            flatten_expression_into(working_set, &cell_path.head, output);
            output.extend(cell_path.tail.iter().map(|member| match *member {
                PathMember::String { span, .. } => (span, FlatShape::String),
                PathMember::Int { span, .. } => (span, FlatShape::Int),
            }));
        }
        Expr::ImportPattern(import_pattern) => {
            output.push((import_pattern.head.span, FlatShape::String));

            for member in &import_pattern.members {
                match member {
                    ImportPatternMember::Glob { span } => output.push((*span, FlatShape::String)),
                    ImportPatternMember::Name { span, .. } => {
                        output.push((*span, FlatShape::String))
                    }
                    ImportPatternMember::List { names } => {
                        output.extend(names.iter().map(|&(_, span)| (span, FlatShape::String)))
                    }
                }
            }
        }
        Expr::Overlay(_) => output.push((expr.span, FlatShape::String)),
        Expr::Range(range) => {
            if let Some(f) = &range.from {
                flatten_expression_into(working_set, f, output);
            }
            if let Some(s) = &range.next {
                output.push((range.operator.next_op_span, FlatShape::Operator));
                flatten_expression_into(working_set, s, output);
            }
            output.push((range.operator.span, FlatShape::Operator));
            if let Some(t) = &range.to {
                flatten_expression_into(working_set, t, output);
            }
        }
        Expr::Bool(_) => output.push((expr.span, FlatShape::Bool)),
        Expr::Filepath(_, _) => output.push((expr.span, FlatShape::Filepath)),
        Expr::Directory(_, _) => output.push((expr.span, FlatShape::Directory)),
        Expr::GlobPattern(_, _) => output.push((expr.span, FlatShape::GlobPattern)),
        Expr::List(list) => {
            let outer_span = expr.span;
            let mut last_end = outer_span.start;

            for item in list {
                match item {
                    ListItem::Item(expr) => {
                        let flattened = flatten_expression(working_set, expr);

                        if let Some(first) = flattened.first()
                            && first.0.start > last_end
                        {
                            output.push((Span::new(last_end, first.0.start), FlatShape::List));
                        }

                        if let Some(last) = flattened.last() {
                            last_end = last.0.end;
                        }

                        output.extend(flattened);
                    }
                    ListItem::Spread(op_span, expr) => {
                        if op_span.start > last_end {
                            output.push((Span::new(last_end, op_span.start), FlatShape::List));
                        }
                        output.push((*op_span, FlatShape::Operator));
                        last_end = op_span.end;

                        let flattened_inner = flatten_expression(working_set, expr);
                        if let Some(first) = flattened_inner.first()
                            && first.0.start > last_end
                        {
                            output.push((Span::new(last_end, first.0.start), FlatShape::List));
                        }
                        if let Some(last) = flattened_inner.last() {
                            last_end = last.0.end;
                        }
                        output.extend(flattened_inner);
                    }
                }
            }

            if last_end < outer_span.end {
                output.push((Span::new(last_end, outer_span.end), FlatShape::List));
            }
        }
        Expr::StringInterpolation(exprs) => {
            let mut flattened = vec![];
            for expr in exprs {
                flatten_expression_into(working_set, expr, &mut flattened);
            }

            if let Some(first) = flattened.first()
                && first.0.start != expr.span.start
            {
                // If we aren't a bare word interpolation, also highlight the outer quotes
                output.push((
                    Span::new(expr.span.start, expr.span.start + 2),
                    FlatShape::StringInterpolation,
                ));
                flattened.push((
                    Span::new(expr.span.end - 1, expr.span.end),
                    FlatShape::StringInterpolation,
                ));
            }
            output.extend(flattened);
        }
        Expr::GlobInterpolation(exprs, quoted) => {
            let mut flattened = vec![];
            for expr in exprs {
                flatten_expression_into(working_set, expr, &mut flattened);
            }

            if *quoted {
                // If we aren't a bare word interpolation, also highlight the outer quotes
                output.push((
                    Span::new(expr.span.start, expr.span.start + 2),
                    FlatShape::GlobInterpolation,
                ));
                flattened.push((
                    Span::new(expr.span.end - 1, expr.span.end),
                    FlatShape::GlobInterpolation,
                ));
            }
            output.extend(flattened);
        }
        Expr::Record(list) => {
            let outer_span = expr.span;
            let mut last_end = outer_span.start;

            for l in list {
                match l {
                    RecordItem::Pair(key, val) => {
                        let flattened_lhs = flatten_expression(working_set, key);
                        let flattened_rhs = flatten_expression(working_set, val);

                        if let Some(first) = flattened_lhs.first()
                            && first.0.start > last_end
                        {
                            output.push((Span::new(last_end, first.0.start), FlatShape::Record));
                        }
                        if let Some(last) = flattened_lhs.last() {
                            last_end = last.0.end;
                        }
                        output.extend(flattened_lhs);

                        if let Some(first) = flattened_rhs.first()
                            && first.0.start > last_end
                        {
                            output.push((Span::new(last_end, first.0.start), FlatShape::Record));
                        }
                        if let Some(last) = flattened_rhs.last() {
                            last_end = last.0.end;
                        }

                        output.extend(flattened_rhs);
                    }
                    RecordItem::Spread(op_span, record) => {
                        if op_span.start > last_end {
                            output.push((Span::new(last_end, op_span.start), FlatShape::Record));
                        }
                        output.push((*op_span, FlatShape::Operator));
                        last_end = op_span.end;

                        let flattened = flatten_expression(working_set, record);
                        if let Some(first) = flattened.first()
                            && first.0.start > last_end
                        {
                            output.push((Span::new(last_end, first.0.start), FlatShape::Record));
                        }
                        if let Some(last) = flattened.last() {
                            last_end = last.0.end;
                        }
                        output.extend(flattened);
                    }
                }
            }
            if last_end < outer_span.end {
                output.push((Span::new(last_end, outer_span.end), FlatShape::Record));
            }
        }
        Expr::Keyword(kw) => {
            output.push((kw.span, FlatShape::Keyword));
            flatten_expression_into(working_set, &kw.expr, output);
        }
        Expr::Operator(_) => output.push((expr.span, FlatShape::Operator)),
        Expr::Signature(_) => output.push((expr.span, FlatShape::Signature)),
        Expr::String(_) => output.push((expr.span, FlatShape::String)),
        Expr::RawString(_) => output.push((expr.span, FlatShape::RawString)),
        Expr::Table(table) => {
            let outer_span = expr.span;
            let mut last_end = outer_span.start;

            for col in table.columns.as_ref() {
                let flattened = flatten_expression(working_set, col);
                if let Some(first) = flattened.first()
                    && first.0.start > last_end
                {
                    output.push((Span::new(last_end, first.0.start), FlatShape::Table));
                }

                if let Some(last) = flattened.last() {
                    last_end = last.0.end;
                }

                output.extend(flattened);
            }
            for row in table.rows.as_ref() {
                for expr in row.as_ref() {
                    let flattened = flatten_expression(working_set, expr);
                    if let Some(first) = flattened.first()
                        && first.0.start > last_end
                    {
                        output.push((Span::new(last_end, first.0.start), FlatShape::Table));
                    }

                    if let Some(last) = flattened.last() {
                        last_end = last.0.end;
                    }

                    output.extend(flattened);
                }
            }

            if last_end < outer_span.end {
                output.push((Span::new(last_end, outer_span.end), FlatShape::Table));
            }
        }
        Expr::Var(var_id) => output.push((expr.span, FlatShape::Variable(*var_id))),
        Expr::VarDecl(var_id) => output.push((expr.span, FlatShape::VarDecl(*var_id))),
    }
}

fn flatten_pattern_into(match_pattern: &MatchPattern, output: &mut Vec<(Span, FlatShape)>) {
    match &match_pattern.pattern {
        Pattern::Garbage => output.push((match_pattern.span, FlatShape::Garbage)),
        Pattern::IgnoreValue => output.push((match_pattern.span, FlatShape::Nothing)),
        Pattern::IgnoreRest => output.push((match_pattern.span, FlatShape::Nothing)),
        Pattern::List(items) => {
            if let Some(first) = items.first() {
                if let Some(last) = items.last() {
                    output.push((
                        Span::new(match_pattern.span.start, first.span.start),
                        FlatShape::MatchPattern,
                    ));
                    for item in items {
                        flatten_pattern_into(item, output);
                    }
                    output.push((
                        Span::new(last.span.end, match_pattern.span.end),
                        FlatShape::MatchPattern,
                    ))
                }
            } else {
                output.push((match_pattern.span, FlatShape::MatchPattern));
            }
        }
        Pattern::Record(items) => {
            if let Some(first) = items.first() {
                if let Some(last) = items.last() {
                    output.push((
                        Span::new(match_pattern.span.start, first.1.span.start),
                        FlatShape::MatchPattern,
                    ));
                    for (_, pattern) in items {
                        flatten_pattern_into(pattern, output);
                    }
                    output.push((
                        Span::new(last.1.span.end, match_pattern.span.end),
                        FlatShape::MatchPattern,
                    ))
                }
            } else {
                output.push((match_pattern.span, FlatShape::MatchPattern));
            }
        }
        Pattern::Expression(_) | Pattern::Value(_) => {
            output.push((match_pattern.span, FlatShape::MatchPattern))
        }
        Pattern::Variable(var_id) => output.push((match_pattern.span, FlatShape::VarDecl(*var_id))),
        Pattern::Rest(var_id) => output.push((match_pattern.span, FlatShape::VarDecl(*var_id))),
        Pattern::Or(patterns) => {
            for pattern in patterns {
                flatten_pattern_into(pattern, output);
            }
        }
    }
}

pub fn flatten_block(working_set: &StateWorkingSet, block: &Block) -> Vec<(Span, FlatShape)> {
    let mut output = Vec::new();
    flatten_block_into(working_set, block, &mut output);
    output
}

pub fn flatten_pipeline(
    working_set: &StateWorkingSet,
    pipeline: &Pipeline,
) -> Vec<(Span, FlatShape)> {
    let mut output = Vec::new();
    flatten_pipeline_into(working_set, pipeline, &mut output);
    output
}

pub fn flatten_pipeline_element(
    working_set: &StateWorkingSet,
    pipeline_element: &PipelineElement,
) -> Vec<(Span, FlatShape)> {
    let mut output = Vec::new();
    flatten_pipeline_element_into(working_set, pipeline_element, &mut output);
    output
}

pub fn flatten_expression(
    working_set: &StateWorkingSet,
    expr: &Expression,
) -> Vec<(Span, FlatShape)> {
    let mut output = Vec::new();
    flatten_expression_into(working_set, expr, &mut output);
    output
}
