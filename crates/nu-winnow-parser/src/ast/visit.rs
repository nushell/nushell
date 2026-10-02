//! A visitor for walking the AST.
//!
//! Implement [`Visitor`] and override the `visit_*` methods you care about;
//! each default implementation calls the matching `walk_*` function, which
//! visits the node's children. This is the easiest way to build tools such as
//! formatters, linters or symbol indexes on top of the tree.

use super::*;

/// Walks every node of an AST.
#[allow(unused_variables)]
pub trait Visitor<'a> {
    /// Visit a block.
    fn visit_block(&mut self, block: &Block<'a>) {
        walk_block(self, block);
    }
    /// Visit a pipeline.
    fn visit_pipeline(&mut self, pipeline: &Pipeline<'a>) {
        walk_pipeline(self, pipeline);
    }
    /// Visit a pipeline element.
    fn visit_pipeline_element(&mut self, element: &PipelineElement<'a>) {
        walk_pipeline_element(self, element);
    }
    /// Visit an expression.
    fn visit_expression(&mut self, expr: &Expression<'a>) {
        walk_expression(self, expr);
    }
    /// Visit a signature.
    fn visit_signature(&mut self, sig: &Signature<'a>) {
        walk_signature(self, sig);
    }
    /// Visit a parameter.
    fn visit_parameter(&mut self, param: &Parameter<'a>) {
        walk_parameter(self, param);
    }
    /// Visit a type annotation.
    fn visit_type_annotation(&mut self, ty: &TypeAnnotation<'a>) {
        walk_type_annotation(self, ty);
    }
    /// Visit a match pattern.
    fn visit_match_pattern(&mut self, pattern: &MatchPattern<'a>) {
        walk_match_pattern(self, pattern);
    }
    /// Visit a cell-path member.
    fn visit_path_member(&mut self, member: &PathMember<'a>) {}
    /// Visit a comment attached to a pipeline or parameter.
    fn visit_comment(&mut self, comment: &Comment) {}
    /// Visit a redirection.
    fn visit_redirection(&mut self, redirection: &PipelineRedirection<'a>) {
        walk_redirection(self, redirection);
    }
}

/// Visit the children of a block.
pub fn walk_block<'a, V: Visitor<'a> + ?Sized>(visitor: &mut V, block: &Block<'a>) {
    for pipeline in &block.pipelines {
        visitor.visit_pipeline(pipeline);
    }
}

/// Visit the children of a pipeline.
pub fn walk_pipeline<'a, V: Visitor<'a> + ?Sized>(visitor: &mut V, pipeline: &Pipeline<'a>) {
    for comment in &pipeline.leading_comments {
        visitor.visit_comment(comment);
    }
    for element in &pipeline.elements {
        visitor.visit_pipeline_element(element);
    }
    for comment in &pipeline.trailing_comments {
        visitor.visit_comment(comment);
    }
}

/// Visit the children of a pipeline element.
pub fn walk_pipeline_element<'a, V: Visitor<'a> + ?Sized>(visitor: &mut V, element: &PipelineElement<'a>) {
    visitor.visit_expression(&element.expr);
    if let Some(redirection) = &element.redirection {
        visitor.visit_redirection(redirection);
    }
}

/// Visit the children of a redirection.
pub fn walk_redirection<'a, V: Visitor<'a> + ?Sized>(visitor: &mut V, redirection: &PipelineRedirection<'a>) {
    let mut visit_target = |target: &RedirectionTarget<'a>| {
        if let RedirectionTarget::File { path, .. } = target {
            visitor.visit_expression(path);
        }
    };
    match redirection {
        PipelineRedirection::Single { target, .. } => visit_target(target),
        PipelineRedirection::Separate { out, err } => {
            visit_target(out);
            visit_target(err);
        }
    }
}

/// Visit the children of an expression.
pub fn walk_expression<'a, V: Visitor<'a> + ?Sized>(visitor: &mut V, expr: &Expression<'a>) {
    match &expr.expr {
        Expr::Bool(_)
        | Expr::Nothing
        | Expr::Int(_)
        | Expr::Float(_)
        | Expr::String(_)
        | Expr::Binary(_)
        | Expr::Duration(_)
        | Expr::Filesize(_)
        | Expr::DateTime(_)
        | Expr::Var(_)
        | Expr::Break
        | Expr::Continue
        | Expr::Garbage => {}
        Expr::StringInterpolation(interpolation) => {
            for part in &interpolation.parts {
                if let InterpolationPart::Expression(expr) = part {
                    visitor.visit_expression(expr);
                }
            }
        }
        Expr::Range(range) => {
            if let Some(from) = &range.from {
                visitor.visit_expression(from);
            }
            if let Some(next) = &range.next {
                visitor.visit_expression(next);
            }
            if let Some(to) = &range.to {
                visitor.visit_expression(to);
            }
        }
        Expr::CellPath(cell_path) => {
            for member in &cell_path.members {
                visitor.visit_path_member(member);
            }
        }
        Expr::FullCellPath(full_cell_path) => {
            visitor.visit_expression(&full_cell_path.head);
            for member in &full_cell_path.tail {
                visitor.visit_path_member(member);
            }
        }
        Expr::List(items) => {
            for item in items {
                match item {
                    ListItem::Item(expr) | ListItem::Spread { expr, .. } => visitor.visit_expression(expr),
                }
            }
        }
        Expr::Table(table) => {
            visitor.visit_expression(&table.columns);
            for row in &table.rows {
                visitor.visit_expression(row);
            }
        }
        Expr::Record(items) => {
            for item in items {
                match item {
                    RecordItem::Pair { key, value, .. } => {
                        visitor.visit_expression(key);
                        visitor.visit_expression(value);
                    }
                    RecordItem::Spread { expr, .. } => visitor.visit_expression(expr),
                }
            }
        }
        Expr::Closure(closure) => {
            if let Some(sig) = &closure.params {
                visitor.visit_signature(sig);
            }
            visitor.visit_block(&closure.body);
        }
        Expr::Block(block) | Expr::Subexpression(block) => visitor.visit_block(block),
        Expr::BinaryOp(binary) => {
            visitor.visit_expression(&binary.lhs);
            visitor.visit_expression(&binary.rhs);
        }
        Expr::UnaryNot(not) => visitor.visit_expression(&not.expr),
        Expr::Assignment(assignment) => {
            visitor.visit_expression(&assignment.lhs);
            visitor.visit_block(&assignment.rhs);
        }
        Expr::Call(call) => walk_arguments(visitor, &call.arguments),
        Expr::DynamicCall(dynamic_call) => {
            visitor.visit_expression(&dynamic_call.head);
            walk_arguments(visitor, &dynamic_call.arguments);
        }
        Expr::ExternalCall(external_call) => {
            visitor.visit_expression(&external_call.head);
            for arg in &external_call.arguments {
                match arg {
                    ExternalArgument::Regular(expr) | ExternalArgument::Spread { expr, .. } => {
                        visitor.visit_expression(expr)
                    }
                }
            }
        }
        Expr::EnvShorthand(env_shorthand) => {
            for var in &env_shorthand.vars {
                visitor.visit_expression(&var.value);
            }
            visitor.visit_expression(&env_shorthand.expr);
        }
        Expr::AttributeBlock(attribute_block) => {
            for attr in &attribute_block.attributes {
                walk_arguments(visitor, &attr.arguments);
            }
            visitor.visit_expression(&attribute_block.item);
        }
        Expr::Let(binding) | Expr::Mut(binding) | Expr::Const(binding) => {
            if let Some(ty) = &binding.ty {
                visitor.visit_type_annotation(ty);
            }
            if let Some(value) = &binding.value {
                visitor.visit_block(value);
            }
        }
        Expr::Def(def) => {
            visitor.visit_signature(&def.signature);
            if let Some(body_params) = &def.body_params {
                visitor.visit_signature(body_params);
            }
            visitor.visit_block(&def.body);
        }
        Expr::Extern(extern_declaration) => visitor.visit_signature(&extern_declaration.signature),
        Expr::Alias(alias) => {
            if let Some(v_) = &alias.value {
                visitor.visit_expression(v_);
            }
        }
        Expr::Use(use_statement) => {
            visitor.visit_expression(&use_statement.module);
            for member in &use_statement.members {
                if let ImportPatternMemberKind::Ignored(ignored) = &member.kind {
                    visitor.visit_expression(ignored);
                }
            }
        }
        Expr::Module(module) => {
            visitor.visit_expression(&module.name);
            if let Some(body) = &module.body {
                visitor.visit_block(body);
            }
        }
        Expr::Export(export) => visitor.visit_expression(&export.item),
        Expr::ExportEnv(export_env) => visitor.visit_block(&export_env.body),
        Expr::If(if_expression) => {
            visitor.visit_expression(&if_expression.condition);
            walk_block_or_value(visitor, &if_expression.then_block, if_expression.then_value.as_deref());
            if let Some(else_branch) = &if_expression.else_branch {
                visitor.visit_expression(&else_branch.body);
            }
        }
        Expr::Match(match_expression) => {
            visitor.visit_expression(&match_expression.value);
            for arm in &match_expression.arms {
                visitor.visit_match_pattern(&arm.pattern);
                if let Some(guard) = &arm.guard {
                    visitor.visit_expression(guard);
                }
                visitor.visit_expression(&arm.body);
            }
            if let Some(value_block) = &match_expression.value_block {
                visitor.visit_expression(value_block);
            }
        }
        Expr::For(for_loop) => {
            if let Some(ty) = &for_loop.ty {
                visitor.visit_type_annotation(ty);
            }
            visitor.visit_expression(&for_loop.iterable);
            walk_block_or_value(visitor, &for_loop.body, for_loop.body_value.as_deref());
        }
        Expr::While(while_loop) => {
            visitor.visit_expression(&while_loop.condition);
            walk_block_or_value(visitor, &while_loop.body, while_loop.body_value.as_deref());
        }
        Expr::Loop(loop_expression) => {
            walk_block_or_value(visitor, &loop_expression.body, loop_expression.body_value.as_deref())
        }
        Expr::Return(return_expression) => {
            if let Some(value) = &return_expression.value {
                visitor.visit_expression(value);
            }
        }
        Expr::Try(try_expression) => {
            walk_block_or_value(visitor, &try_expression.body, try_expression.body_value.as_deref());
            for handler in &try_expression.handlers {
                visitor.visit_expression(&handler.body);
            }
        }
        Expr::Where(where_expression) => visitor.visit_expression(&where_expression.condition),
    }
}

/// A keyword's block, or the variable or subexpression standing in for it.
fn walk_block_or_value<'a, V: Visitor<'a> + ?Sized>(
    visitor: &mut V,
    block: &Block<'a>,
    value: Option<&Expression<'a>>,
) {
    match value {
        Some(value) => visitor.visit_expression(value),
        None => visitor.visit_block(block),
    }
}

fn walk_arguments<'a, V: Visitor<'a> + ?Sized>(visitor: &mut V, args: &[Argument<'a>]) {
    for arg in args {
        match arg {
            Argument::Positional(expr) | Argument::Spread { expr, .. } => visitor.visit_expression(expr),
            Argument::Named(flag) => {
                if let Some(value) = &flag.value {
                    visitor.visit_expression(value);
                }
            }
            Argument::EndOfOptions(_) => {}
        }
    }
}

/// Visit the children of a signature.
pub fn walk_signature<'a, V: Visitor<'a> + ?Sized>(visitor: &mut V, sig: &Signature<'a>) {
    for parameter in &sig.params {
        visitor.visit_parameter(parameter);
    }
    for io in &sig.input_output_types {
        visitor.visit_type_annotation(&io.input);
        visitor.visit_type_annotation(&io.output);
    }
}

/// Visit the children of a parameter.
pub fn walk_parameter<'a, V: Visitor<'a> + ?Sized>(visitor: &mut V, param: &Parameter<'a>) {
    if let Some(ty) = &param.ty {
        visitor.visit_type_annotation(ty);
    }
    if let Some(default) = &param.default {
        visitor.visit_expression(default);
    }
    for comment in &param.description {
        visitor.visit_comment(comment);
    }
}

/// Visit the children of a type annotation.
pub fn walk_type_annotation<'a, V: Visitor<'a> + ?Sized>(visitor: &mut V, ty: &TypeAnnotation<'a>) {
    match &ty.shape {
        SyntaxShape::List(Some(inner)) => visitor.visit_type_annotation(inner),
        SyntaxShape::Record(fields) | SyntaxShape::Table(fields) => {
            for field in fields {
                visitor.visit_type_annotation(&field.ty);
            }
        }
        SyntaxShape::OneOf(types) => {
            for ty in types {
                visitor.visit_type_annotation(ty);
            }
        }
        _ => {}
    }
}

/// Visit the children of a pattern.
pub fn walk_match_pattern<'a, V: Visitor<'a> + ?Sized>(visitor: &mut V, pattern: &MatchPattern<'a>) {
    match &pattern.pattern {
        Pattern::Expression(expr) => visitor.visit_expression(expr),
        Pattern::Variable(_) | Pattern::IgnoreValue | Pattern::Rest(_) | Pattern::IgnoreRest => {}
        Pattern::List(items) | Pattern::Or(items) => {
            for item in items {
                visitor.visit_match_pattern(item);
            }
        }
        Pattern::Record(fields) => {
            for (_, pattern) in fields {
                visitor.visit_match_pattern(pattern);
            }
        }
    }
}
