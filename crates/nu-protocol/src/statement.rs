use crate::{DeclId, Expression, Pipeline};

#[derive(Debug, Clone)]
pub enum Statement {
    Declaration(DeclId),
    Pipeline(Pipeline),
    Expression(Expression),
}
