use crate::{Block, Expr, Expression, ParserWorkingSet, Pipeline, Span, Statement};

#[derive(Debug)]
pub enum FlatShape {
    Garbage,
    Bool,
    Int,
    InternalCall,
    External,
    Literal,
    Operator,
    Signature,
    String,
    Variable,
}

impl<'a> ParserWorkingSet<'a> {
    pub fn flatten_block(&self, block: &Block) -> Vec<(Span, FlatShape)> {
        let mut output = vec![];
        for stmt in &block.stmts {
            output.extend(self.flatten_statement(stmt));
        }
        output
    }

    pub fn flatten_statement(&self, stmt: &Statement) -> Vec<(Span, FlatShape)> {
        match stmt {
            Statement::Expression(expr) => self.flatten_expression(expr),
            Statement::Pipeline(pipeline) => self.flatten_pipeline(pipeline),
            _ => vec![],
        }
    }

    pub fn flatten_expression(&self, expr: &Expression) -> Vec<(Span, FlatShape)> {
        match &expr.expr {
            Expr::BinaryOp(lhs, op, rhs) => {
                let mut output = vec![];
                output.extend(self.flatten_expression(lhs));
                output.extend(self.flatten_expression(op));
                output.extend(self.flatten_expression(rhs));
                output
            }
            Expr::Block(block_id) => self.flatten_block(self.get_block(*block_id)),
            Expr::Call(call) => {
                let mut output = vec![(call.head, FlatShape::InternalCall)];
                for positional in &call.positional {
                    output.extend(self.flatten_expression(positional));
                }
                output
            }
            Expr::ExternalCall(..) => {
                vec![(expr.span, FlatShape::External)]
            }
            Expr::Garbage => {
                vec![(expr.span, FlatShape::Garbage)]
            }
            Expr::Int(_) => {
                vec![(expr.span, FlatShape::Int)]
            }
            Expr::Bool(_) => {
                vec![(expr.span, FlatShape::Bool)]
            }

            Expr::List(list) => {
                let mut output = vec![];
                for l in list {
                    output.extend(self.flatten_expression(l));
                }
                output
            }
            Expr::Keyword(_, span, expr) => {
                let mut output = vec![(*span, FlatShape::Operator)];
                output.extend(self.flatten_expression(expr));
                output
            }
            Expr::Operator(_) => {
                vec![(expr.span, FlatShape::Operator)]
            }
            Expr::Signature(_) => {
                vec![(expr.span, FlatShape::Signature)]
            }
            Expr::String(_) => {
                vec![(expr.span, FlatShape::String)]
            }
            Expr::Subexpression(block_id) => self.flatten_block(self.get_block(*block_id)),
            Expr::Table(headers, cells) => {
                let mut output = vec![];
                for e in headers {
                    output.extend(self.flatten_expression(e));
                }
                for row in cells {
                    for expr in row {
                        output.extend(self.flatten_expression(expr));
                    }
                }
                output
            }
            Expr::Var(_) => {
                vec![(expr.span, FlatShape::Variable)]
            }
        }
    }

    pub fn flatten_pipeline(&self, pipeline: &Pipeline) -> Vec<(Span, FlatShape)> {
        let mut output = vec![];
        for expr in &pipeline.expressions {
            output.extend(self.flatten_expression(expr))
        }
        output
    }
}
