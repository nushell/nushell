use super::{Call, Expression, FullCellPath, Operator, RangeOperator};
use crate::{BlockId, Signature, Span, VarId};

#[derive(Debug, Clone)]
pub enum Expr {
    Bool(bool),
    Int(i64),
    Float(f64),
    Range(
        Option<Box<Expression>>, // from
        Option<Box<Expression>>, // next value after "from"
        Option<Box<Expression>>, // to
        RangeOperator,
    ),
    Var(VarId),
    Call(Box<Call>),
    ExternalCall(Span, Vec<Span>),
    Operator(Operator),
    RowCondition(VarId, Box<Expression>),
    BinaryOp(Box<Expression>, Box<Expression>, Box<Expression>), //lhs, op, rhs
    Subexpression(BlockId),
    Block(BlockId),
    List(Vec<Expression>),
    Table(Vec<Expression>, Vec<Vec<Expression>>),
    Keyword(Vec<u8>, Span, Box<Expression>),
    String(String), // FIXME: improve this in the future?
    FullCellPath(Box<FullCellPath>),
    Signature(Box<Signature>),
    Garbage,
}
