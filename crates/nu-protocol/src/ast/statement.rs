use super::{Expression, Pipeline};
use crate::DeclId;

#[derive(Debug, Clone)]
pub enum Statement {
    Declaration(DeclId),
    Pipeline(Pipeline),
    Expression(Expression),
}
