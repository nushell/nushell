use crate::{parser::Operator, Block, Expr, Expression, Span, Statement};

#[derive(Debug)]
pub enum ShellError {
    Mismatch(String, Span),
    Unsupported(Span),
}

pub struct Engine;

#[derive(Debug)]
pub enum Value {
    Int { val: i64, span: Span },
    Unknown,
}

impl Value {
    pub fn add(&self, rhs: &Value) -> Result<Value, ShellError> {
        match (self, rhs) {
            (Value::Int { val: lhs, .. }, Value::Int { val: rhs, .. }) => Ok(Value::Int {
                val: lhs + rhs,
                span: Span::unknown(),
            }),
            _ => Ok(Value::Unknown),
        }
    }
}

impl Default for Engine {
    fn default() -> Self {
        Self::new()
    }
}

impl Engine {
    pub fn new() -> Self {
        Self
    }

    pub fn eval_operator(&self, op: &Expression) -> Result<Operator, ShellError> {
        match op {
            Expression {
                expr: Expr::Operator(operator),
                ..
            } => Ok(operator.clone()),
            Expression { span, .. } => Err(ShellError::Mismatch("operator".to_string(), *span)),
        }
    }

    pub fn eval_expression(&self, expr: &Expression) -> Result<Value, ShellError> {
        match expr.expr {
            Expr::Int(i) => Ok(Value::Int {
                val: i,
                span: expr.span,
            }),
            Expr::Var(_) => Err(ShellError::Unsupported(expr.span)),
            Expr::Call(_) => Err(ShellError::Unsupported(expr.span)),
            Expr::ExternalCall(_, _) => Err(ShellError::Unsupported(expr.span)),
            Expr::Operator(_) => Err(ShellError::Unsupported(expr.span)),
            Expr::BinaryOp(_, _, _) => Err(ShellError::Unsupported(expr.span)),
            Expr::Subexpression(_) => Err(ShellError::Unsupported(expr.span)),
            Expr::Block(_) => Err(ShellError::Unsupported(expr.span)),
            Expr::List(_) => Err(ShellError::Unsupported(expr.span)),
            Expr::Table(_, _) => Err(ShellError::Unsupported(expr.span)),
            Expr::Literal(_) => Err(ShellError::Unsupported(expr.span)),
            Expr::String(_) => Err(ShellError::Unsupported(expr.span)),
            Expr::Garbage => Err(ShellError::Unsupported(expr.span)),
        }
    }

    pub fn eval_block(&self, block: &Block) -> Result<Value, ShellError> {
        let mut last = Ok(Value::Unknown);

        for stmt in &block.stmts {
            match stmt {
                Statement::Expression(expression) => match &expression.expr {
                    Expr::BinaryOp(lhs, op, rhs) => {
                        let lhs = self.eval_expression(&lhs)?;
                        let op = self.eval_operator(&op)?;
                        let rhs = self.eval_expression(&rhs)?;

                        match op {
                            Operator::Plus => last = lhs.add(&rhs),
                            _ => {}
                        }
                    }
                    _ => {}
                },
                _ => {}
            }
        }

        last
    }
}
