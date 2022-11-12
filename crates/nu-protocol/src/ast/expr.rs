use chrono::FixedOffset;
use serde::{Deserialize, Serialize};

use super::{Call, CellPath, Expression, FullCellPath, Operator, RangeOperator};
use crate::{ast::ImportPattern, BlockId, Signature, Span, Spanned, Unit, VarId};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum Expr {
    Bool(bool),
    Int(i64),
    Float(f64),
    Binary(Vec<u8>),
    Range(
        Option<Box<Expression>>, // from
        Option<Box<Expression>>, // next value after "from"
        Option<Box<Expression>>, // to
        RangeOperator,
    ),
    Var(VarId),
    VarDecl(VarId),
    Call(Box<Call>),
    ExternalCall(Box<Expression>, Vec<Expression>),
    Operator(Operator),
    RowCondition(BlockId),
    UnaryNot(Box<Expression>),
    BinaryOp(Box<Expression>, Box<Expression>, Box<Expression>), //lhs, op, rhs
    Subexpression(BlockId),
    Block(BlockId),
    Closure(BlockId),
    List(Vec<Expression>),
    Table(Vec<Expression>, Vec<Vec<Expression>>),
    Record(Vec<(Expression, Expression)>),
    Keyword(Vec<u8>, Span, Box<Expression>),
    ValueWithUnit(Box<Expression>, Spanned<Unit>),
    DateTime(chrono::DateTime<FixedOffset>),
    Filepath(String),
    Directory(String),
    GlobPattern(String),
    String(String),
    CellPath(CellPath),
    FullCellPath(Box<FullCellPath>),
    ImportPattern(ImportPattern),
    Overlay(Option<BlockId>), // block ID of the overlay's origin module
    Signature(Box<Signature>),
    StringInterpolation(Vec<Expression>),
    Nothing,
    Garbage,
}
