use crate::{BlockId, Expr, Operator, Signature, Span, Type, VarId};

#[derive(Debug, Clone)]
pub struct Expression {
    pub expr: Expr,
    pub span: Span,
    pub ty: Type,
}
impl Expression {
    pub fn garbage(span: Span) -> Expression {
        Expression {
            expr: Expr::Garbage,
            span,
            ty: Type::Unknown,
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
                    Operator::Or => 40, // TODO: should we have And and Or be different precedence?
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
            _ => None,
        }
    }

    pub fn as_string(&self) -> Option<String> {
        match &self.expr {
            Expr::String(string) => Some(string.clone()),
            _ => None,
        }
    }
}
