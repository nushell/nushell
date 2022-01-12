use super::{Expr, Operator, Statement};
use crate::ast::ImportPattern;
use crate::{engine::StateWorkingSet, BlockId, Signature, Span, Type, VarId, IN_VARIABLE_ID};

#[derive(Debug, Clone)]
pub struct Expression {
    pub expr: Expr,
    pub span: Span,
    pub ty: Type,
    pub custom_completion: Option<String>,
}

impl Expression {
    pub fn garbage(span: Span) -> Expression {
        Expression {
            expr: Expr::Garbage,
            span,
            ty: Type::Unknown,
            custom_completion: None,
        }
    }

    pub fn precedence(&self) -> usize {
        match &self.expr {
            Expr::Operator(operator) => {
                // Higher precedence binds tighter

                match operator {
                    Operator::Pow => 100,
                    Operator::Multiply | Operator::Divide | Operator::Modulo => 95,
                    Operator::Plus | Operator::Minus => 90,
                    Operator::NotContains
                    | Operator::Contains
                    | Operator::LessThan
                    | Operator::LessThanOrEqual
                    | Operator::GreaterThan
                    | Operator::GreaterThanOrEqual
                    | Operator::Equal
                    | Operator::NotEqual
                    | Operator::In
                    | Operator::NotIn => 80,
                    Operator::And => 50,
                    Operator::Or => 40,
                }
            }
            _ => 0,
        }
    }

    pub fn as_block(&self) -> Option<BlockId> {
        match self.expr {
            Expr::Block(block_id) => Some(block_id),
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
            Expr::Block(block_id) => {
                let block = working_set.get_block(*block_id);

                if block.captures.contains(&IN_VARIABLE_ID) {
                    return true;
                }

                if let Some(Statement::Pipeline(pipeline)) = block.stmts.get(0) {
                    match pipeline.expressions.get(0) {
                        Some(expr) => expr.has_in_variable(working_set),
                        None => false,
                    }
                } else {
                    false
                }
            }
            Expr::Bool(_) => false,
            Expr::Call(call) => {
                for positional in &call.positional {
                    if positional.has_in_variable(working_set) {
                        return true;
                    }
                }
                for named in &call.named {
                    if let Some(expr) = &named.1 {
                        if expr.has_in_variable(working_set) {
                            return true;
                        }
                    }
                }
                false
            }
            Expr::CellPath(_) => false,
            Expr::ExternalCall(_, _, args) => {
                for arg in args {
                    if arg.has_in_variable(working_set) {
                        return true;
                    }
                }
                false
            }
            Expr::ImportPattern(_) => false,
            Expr::Filepath(_) => false,
            Expr::Float(_) => false,
            Expr::FullCellPath(full_cell_path) => {
                if full_cell_path.head.has_in_variable(working_set) {
                    return true;
                }
                false
            }
            Expr::Garbage => false,
            Expr::Nothing => false,
            Expr::GlobPattern(_) => false,
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
            Expr::Record(fields) => {
                for (field_name, field_value) in fields {
                    if field_name.has_in_variable(working_set) {
                        return true;
                    }
                    if field_value.has_in_variable(working_set) {
                        return true;
                    }
                }
                false
            }
            Expr::Signature(_) => false,
            Expr::String(_) => false,
            Expr::RowCondition(block_id) | Expr::Subexpression(block_id) => {
                let block = working_set.get_block(*block_id);

                if let Some(Statement::Pipeline(pipeline)) = block.stmts.get(0) {
                    if let Some(expr) = pipeline.expressions.get(0) {
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
        }
    }

    pub fn replace_in_variable(&mut self, working_set: &mut StateWorkingSet, new_var_id: VarId) {
        match &mut self.expr {
            Expr::BinaryOp(left, _, right) => {
                left.replace_in_variable(working_set, new_var_id);
                right.replace_in_variable(working_set, new_var_id);
            }
            Expr::Block(block_id) => {
                let block = working_set.get_block(*block_id);

                let new_expr = if let Some(Statement::Pipeline(pipeline)) = block.stmts.get(0) {
                    if let Some(expr) = pipeline.expressions.get(0) {
                        let mut new_expr = expr.clone();
                        new_expr.replace_in_variable(working_set, new_var_id);
                        Some(new_expr)
                    } else {
                        None
                    }
                } else {
                    None
                };

                let block = working_set.get_block_mut(*block_id);

                if let Some(new_expr) = new_expr {
                    if let Some(Statement::Pipeline(pipeline)) = block.stmts.get_mut(0) {
                        if let Some(expr) = pipeline.expressions.get_mut(0) {
                            *expr = new_expr
                        }
                    }
                }

                block.captures = block
                    .captures
                    .iter()
                    .map(|x| if *x != IN_VARIABLE_ID { *x } else { new_var_id })
                    .collect();
            }
            Expr::Bool(_) => {}
            Expr::Call(call) => {
                for positional in &mut call.positional {
                    positional.replace_in_variable(working_set, new_var_id);
                }
                for named in &mut call.named {
                    if let Some(expr) = &mut named.1 {
                        expr.replace_in_variable(working_set, new_var_id)
                    }
                }
            }
            Expr::CellPath(_) => {}
            Expr::ExternalCall(_, _, args) => {
                for arg in args {
                    arg.replace_in_variable(working_set, new_var_id)
                }
            }
            Expr::Filepath(_) => {}
            Expr::Float(_) => {}
            Expr::FullCellPath(full_cell_path) => {
                full_cell_path
                    .head
                    .replace_in_variable(working_set, new_var_id);
            }
            Expr::ImportPattern(_) => {}
            Expr::Garbage => {}
            Expr::Nothing => {}
            Expr::GlobPattern(_) => {}
            Expr::Int(_) => {}
            Expr::Keyword(_, _, expr) => expr.replace_in_variable(working_set, new_var_id),
            Expr::List(list) => {
                for l in list {
                    l.replace_in_variable(working_set, new_var_id)
                }
            }
            Expr::Operator(_) => {}
            Expr::Range(left, middle, right, ..) => {
                if let Some(left) = left {
                    left.replace_in_variable(working_set, new_var_id)
                }
                if let Some(middle) = middle {
                    middle.replace_in_variable(working_set, new_var_id)
                }
                if let Some(right) = right {
                    right.replace_in_variable(working_set, new_var_id)
                }
            }
            Expr::Record(fields) => {
                for (field_name, field_value) in fields {
                    field_name.replace_in_variable(working_set, new_var_id);
                    field_value.replace_in_variable(working_set, new_var_id);
                }
            }
            Expr::Signature(_) => {}
            Expr::String(_) => {}
            Expr::StringInterpolation(items) => {
                for i in items {
                    i.replace_in_variable(working_set, new_var_id)
                }
            }
            Expr::RowCondition(block_id) | Expr::Subexpression(block_id) => {
                let block = working_set.get_block(*block_id);

                let new_expr = if let Some(Statement::Pipeline(pipeline)) = block.stmts.get(0) {
                    if let Some(expr) = pipeline.expressions.get(0) {
                        let mut new_expr = expr.clone();
                        new_expr.replace_in_variable(working_set, new_var_id);
                        Some(new_expr)
                    } else {
                        None
                    }
                } else {
                    None
                };

                let block = working_set.get_block_mut(*block_id);

                if let Some(new_expr) = new_expr {
                    if let Some(Statement::Pipeline(pipeline)) = block.stmts.get_mut(0) {
                        if let Some(expr) = pipeline.expressions.get_mut(0) {
                            *expr = new_expr
                        }
                    }
                }

                block.captures = block
                    .captures
                    .iter()
                    .map(|x| if *x != IN_VARIABLE_ID { *x } else { new_var_id })
                    .collect();
            }
            Expr::Table(headers, cells) => {
                for header in headers {
                    header.replace_in_variable(working_set, new_var_id)
                }

                for row in cells {
                    for cell in row.iter_mut() {
                        cell.replace_in_variable(working_set, new_var_id)
                    }
                }
            }

            Expr::ValueWithUnit(expr, _) => expr.replace_in_variable(working_set, new_var_id),
            Expr::Var(x) => {
                if *x == IN_VARIABLE_ID {
                    *x = new_var_id
                }
            }
            Expr::VarDecl(_) => {}
        }
    }
}
