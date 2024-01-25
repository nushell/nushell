use serde::{Deserialize, Serialize};

use super::{Argument, Expr, ExternalArgument, RecordItem};
use crate::ast::ImportPattern;
use crate::DeclId;
use crate::{engine::StateWorkingSet, BlockId, Signature, Span, Type, VarId, IN_VARIABLE_ID};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Expression {
    pub expr: Expr,
    pub span: Span,
    pub ty: Type,
    pub custom_completion: Option<DeclId>,
}

impl Expression {
    pub fn garbage(span: Span) -> Expression {
        Expression {
            expr: Expr::Garbage,
            span,
            ty: Type::Any,
            custom_completion: None,
        }
    }

    pub fn precedence(&self) -> usize {
        match &self.expr {
            Expr::Operator(operator) => {
                use super::operator::*;
                // Higher precedence binds tighter

                match operator {
                    Operator::Math(Math::Pow) => 100,
                    Operator::Math(Math::Multiply)
                    | Operator::Math(Math::Divide)
                    | Operator::Math(Math::Modulo)
                    | Operator::Math(Math::FloorDivision) => 95,
                    Operator::Math(Math::Plus) | Operator::Math(Math::Minus) => 90,
                    Operator::Bits(Bits::ShiftLeft) | Operator::Bits(Bits::ShiftRight) => 85,
                    Operator::Comparison(Comparison::NotRegexMatch)
                    | Operator::Comparison(Comparison::RegexMatch)
                    | Operator::Comparison(Comparison::StartsWith)
                    | Operator::Comparison(Comparison::EndsWith)
                    | Operator::Comparison(Comparison::LessThan)
                    | Operator::Comparison(Comparison::LessThanOrEqual)
                    | Operator::Comparison(Comparison::GreaterThan)
                    | Operator::Comparison(Comparison::GreaterThanOrEqual)
                    | Operator::Comparison(Comparison::Equal)
                    | Operator::Comparison(Comparison::NotEqual)
                    | Operator::Comparison(Comparison::In)
                    | Operator::Comparison(Comparison::NotIn)
                    | Operator::Math(Math::Append) => 80,
                    Operator::Bits(Bits::BitAnd) => 75,
                    Operator::Bits(Bits::BitXor) => 70,
                    Operator::Bits(Bits::BitOr) => 60,
                    Operator::Boolean(Boolean::And) => 50,
                    Operator::Boolean(Boolean::Xor) => 45,
                    Operator::Boolean(Boolean::Or) => 40,
                    Operator::Assignment(_) => 10,
                }
            }
            _ => 0,
        }
    }

    pub fn as_block(&self) -> Option<BlockId> {
        match self.expr {
            Expr::Block(block_id) => Some(block_id),
            Expr::Closure(block_id) => Some(block_id),
            _ => None,
        }
    }

    pub fn as_row_condition_block(&self) -> Option<BlockId> {
        match self.expr {
            Expr::RowCondition(block_id) => Some(block_id),
            _ => None,
        }
    }

    pub fn as_signature(&self) -> Option<Box<Signature>> {
        match &self.expr {
            Expr::Signature(sig) => Some(sig.clone()),
            _ => None,
        }
    }

    pub fn as_list(&self) -> Option<Vec<Expression>> {
        match &self.expr {
            Expr::List(list) => Some(list.clone()),
            _ => None,
        }
    }

    pub fn as_keyword(&self) -> Option<&Expression> {
        match &self.expr {
            Expr::Keyword(_, _, expr) => Some(expr),
            _ => None,
        }
    }

    pub fn as_var(&self) -> Option<VarId> {
        match self.expr {
            Expr::Var(var_id) => Some(var_id),
            Expr::VarDecl(var_id) => Some(var_id),
            _ => None,
        }
    }

    pub fn as_string(&self) -> Option<String> {
        match &self.expr {
            Expr::String(string) => Some(string.clone()),
            _ => None,
        }
    }

    pub fn as_import_pattern(&self) -> Option<ImportPattern> {
        match &self.expr {
            Expr::ImportPattern(pattern) => Some(pattern.clone()),
            _ => None,
        }
    }

    pub fn has_in_variable(&self, working_set: &StateWorkingSet) -> bool {
        match &self.expr {
            Expr::BinaryOp(left, _, right) => {
                left.has_in_variable(working_set) || right.has_in_variable(working_set)
            }
            Expr::UnaryNot(expr) => expr.has_in_variable(working_set),
            Expr::Block(block_id) => {
                let block = working_set.get_block(*block_id);

                if block.captures.contains(&IN_VARIABLE_ID) {
                    return true;
                }

                if let Some(pipeline) = block.pipelines.first() {
                    match pipeline.elements.first() {
                        Some(element) => element.has_in_variable(working_set),
                        None => false,
                    }
                } else {
                    false
                }
            }
            Expr::Closure(block_id) => {
                let block = working_set.get_block(*block_id);

                if block.captures.contains(&IN_VARIABLE_ID) {
                    return true;
                }

                if let Some(pipeline) = block.pipelines.first() {
                    match pipeline.elements.first() {
                        Some(element) => element.has_in_variable(working_set),
                        None => false,
                    }
                } else {
                    false
                }
            }
            Expr::Binary(_) => false,
            Expr::Bool(_) => false,
            Expr::Call(call) => {
                for arg in &call.arguments {
                    match arg {
                        Argument::Positional(expr)
                        | Argument::Unknown(expr)
                        | Argument::Spread(expr) => {
                            if expr.has_in_variable(working_set) {
                                return true;
                            }
                        }
                        Argument::Named(named) => {
                            if let Some(expr) = &named.2 {
                                if expr.has_in_variable(working_set) {
                                    return true;
                                }
                            }
                        }
                    }
                }
                false
            }
            Expr::CellPath(_) => false,
            Expr::DateTime(_) => false,
            Expr::ExternalCall(head, args, _) => {
                if head.has_in_variable(working_set) {
                    return true;
                }
                for ExternalArgument::Regular(expr) | ExternalArgument::Spread(expr) in args {
                    if expr.has_in_variable(working_set) {
                        return true;
                    }
                }
                false
            }
            Expr::ImportPattern(_) => false,
            Expr::Overlay(_) => false,
            Expr::Filepath(_, _) => false,
            Expr::Directory(_, _) => false,
            Expr::Float(_) => false,
            Expr::FullCellPath(full_cell_path) => {
                if full_cell_path.head.has_in_variable(working_set) {
                    return true;
                }
                false
            }
            Expr::Garbage => false,
            Expr::Nothing => false,
            Expr::GlobPattern(_, _) => false,
            Expr::Int(_) => false,
            Expr::Keyword(_, _, expr) => expr.has_in_variable(working_set),
            Expr::List(list) => {
                for l in list {
                    if l.has_in_variable(working_set) {
                        return true;
                    }
                }
                false
            }
            Expr::StringInterpolation(items) => {
                for i in items {
                    if i.has_in_variable(working_set) {
                        return true;
                    }
                }
                false
            }
            Expr::Operator(_) => false,
            Expr::MatchBlock(_) => false,
            Expr::Range(left, middle, right, ..) => {
                if let Some(left) = &left {
                    if left.has_in_variable(working_set) {
                        return true;
                    }
                }
                if let Some(middle) = &middle {
                    if middle.has_in_variable(working_set) {
                        return true;
                    }
                }
                if let Some(right) = &right {
                    if right.has_in_variable(working_set) {
                        return true;
                    }
                }
                false
            }
            Expr::Record(items) => {
                for item in items {
                    match item {
                        RecordItem::Pair(field_name, field_value) => {
                            if field_name.has_in_variable(working_set) {
                                return true;
                            }
                            if field_value.has_in_variable(working_set) {
                                return true;
                            }
                        }
                        RecordItem::Spread(_, record) => {
                            if record.has_in_variable(working_set) {
                                return true;
                            }
                        }
                    }
                }
                false
            }
            Expr::Signature(_) => false,
            Expr::String(_) => false,
            Expr::RowCondition(block_id) | Expr::Subexpression(block_id) => {
                let block = working_set.get_block(*block_id);

                if let Some(pipeline) = block.pipelines.first() {
                    if let Some(expr) = pipeline.elements.first() {
                        expr.has_in_variable(working_set)
                    } else {
                        false
                    }
                } else {
                    false
                }
            }
            Expr::Table(headers, cells) => {
                for header in headers {
                    if header.has_in_variable(working_set) {
                        return true;
                    }
                }

                for row in cells {
                    for cell in row.iter() {
                        if cell.has_in_variable(working_set) {
                            return true;
                        }
                    }
                }

                false
            }

            Expr::ValueWithUnit(expr, _) => expr.has_in_variable(working_set),
            Expr::Var(var_id) => *var_id == IN_VARIABLE_ID,
            Expr::VarDecl(_) => false,
            Expr::Spread(expr) => expr.has_in_variable(working_set),
        }
    }

    pub fn replace_span(
        &mut self,
        working_set: &mut StateWorkingSet,
        replaced: Span,
        new_span: Span,
    ) {
        if replaced.contains_span(self.span) {
            self.span = new_span;
        }
        match &mut self.expr {
            Expr::BinaryOp(left, _, right) => {
                left.replace_span(working_set, replaced, new_span);
                right.replace_span(working_set, replaced, new_span);
            }
            Expr::UnaryNot(expr) => {
                expr.replace_span(working_set, replaced, new_span);
            }
            Expr::Block(block_id) => {
                let mut block = working_set.get_block(*block_id).clone();

                for pipeline in block.pipelines.iter_mut() {
                    for element in pipeline.elements.iter_mut() {
                        element.replace_span(working_set, replaced, new_span)
                    }
                }

                *block_id = working_set.add_block(block);
            }
            Expr::Closure(block_id) => {
                let mut block = working_set.get_block(*block_id).clone();

                for pipeline in block.pipelines.iter_mut() {
                    for element in pipeline.elements.iter_mut() {
                        element.replace_span(working_set, replaced, new_span)
                    }
                }

                *block_id = working_set.add_block(block);
            }
            Expr::Binary(_) => {}
            Expr::Bool(_) => {}
            Expr::Call(call) => {
                if replaced.contains_span(call.head) {
                    call.head = new_span;
                }
                for arg in call.arguments.iter_mut() {
                    match arg {
                        Argument::Positional(expr)
                        | Argument::Unknown(expr)
                        | Argument::Spread(expr) => {
                            expr.replace_span(working_set, replaced, new_span);
                        }
                        Argument::Named(named) => {
                            if let Some(expr) = &mut named.2 {
                                expr.replace_span(working_set, replaced, new_span);
                            }
                        }
                    }
                }
            }
            Expr::CellPath(_) => {}
            Expr::DateTime(_) => {}
            Expr::ExternalCall(head, args, _) => {
                head.replace_span(working_set, replaced, new_span);
                for ExternalArgument::Regular(expr) | ExternalArgument::Spread(expr) in args {
                    expr.replace_span(working_set, replaced, new_span);
                }
            }
            Expr::Filepath(_, _) => {}
            Expr::Directory(_, _) => {}
            Expr::Float(_) => {}
            Expr::FullCellPath(full_cell_path) => {
                full_cell_path
                    .head
                    .replace_span(working_set, replaced, new_span);
            }
            Expr::ImportPattern(_) => {}
            Expr::Overlay(_) => {}
            Expr::Garbage => {}
            Expr::Nothing => {}
            Expr::GlobPattern(_, _) => {}
            Expr::MatchBlock(_) => {}
            Expr::Int(_) => {}
            Expr::Keyword(_, _, expr) => expr.replace_span(working_set, replaced, new_span),
            Expr::List(list) => {
                for l in list {
                    l.replace_span(working_set, replaced, new_span)
                }
            }
            Expr::Operator(_) => {}
            Expr::Range(left, middle, right, ..) => {
                if let Some(left) = left {
                    left.replace_span(working_set, replaced, new_span)
                }
                if let Some(middle) = middle {
                    middle.replace_span(working_set, replaced, new_span)
                }
                if let Some(right) = right {
                    right.replace_span(working_set, replaced, new_span)
                }
            }
            Expr::Record(items) => {
                for item in items {
                    match item {
                        RecordItem::Pair(field_name, field_value) => {
                            field_name.replace_span(working_set, replaced, new_span);
                            field_value.replace_span(working_set, replaced, new_span);
                        }
                        RecordItem::Spread(_, record) => {
                            record.replace_span(working_set, replaced, new_span);
                        }
                    }
                }
            }
            Expr::Signature(_) => {}
            Expr::String(_) => {}
            Expr::StringInterpolation(items) => {
                for i in items {
                    i.replace_span(working_set, replaced, new_span)
                }
            }
            Expr::RowCondition(block_id) | Expr::Subexpression(block_id) => {
                let mut block = working_set.get_block(*block_id).clone();

                for pipeline in block.pipelines.iter_mut() {
                    for element in pipeline.elements.iter_mut() {
                        element.replace_span(working_set, replaced, new_span)
                    }
                }

                *block_id = working_set.add_block(block);
            }
            Expr::Table(headers, cells) => {
                for header in headers {
                    header.replace_span(working_set, replaced, new_span)
                }

                for row in cells {
                    for cell in row.iter_mut() {
                        cell.replace_span(working_set, replaced, new_span)
                    }
                }
            }

            Expr::ValueWithUnit(expr, _) => expr.replace_span(working_set, replaced, new_span),
            Expr::Var(_) => {}
            Expr::VarDecl(_) => {}
            Expr::Spread(expr) => expr.replace_span(working_set, replaced, new_span),
        }
    }
}
