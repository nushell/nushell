use chrono::FixedOffset;
use serde::{Deserialize, Serialize};

use super::{
    Call, CellPath, Expression, ExternalArgument, FullCellPath, MatchPattern, Operator,
    RangeOperator,
};
use crate::{ast::ImportPattern, ast::Unit, BlockId, Signature, Span, Spanned, VarId};

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
    ExternalCall(Box<Expression>, Vec<ExternalArgument>, bool), // head, args, is_subexpression
    Operator(Operator),
    RowCondition(BlockId),
    UnaryNot(Box<Expression>),
    BinaryOp(Box<Expression>, Box<Expression>, Box<Expression>), //lhs, op, rhs
    Subexpression(BlockId),
    Block(BlockId),
    Closure(BlockId),
    MatchBlock(Vec<(MatchPattern, Expression)>),
    List(Vec<Expression>),
    Table(Vec<Expression>, Vec<Vec<Expression>>),
    Record(Vec<RecordItem>),
    Keyword(Vec<u8>, Span, Box<Expression>),
    ValueWithUnit(Box<Expression>, Spanned<Unit>),
    DateTime(chrono::DateTime<FixedOffset>),
    Filepath(String, bool),
    Directory(String, bool),
    GlobPattern(String, bool),
    String(String),
    CellPath(CellPath),
    FullCellPath(Box<FullCellPath>),
    ImportPattern(ImportPattern),
    Overlay(Option<BlockId>), // block ID of the overlay's origin module
    Signature(Box<Signature>),
    StringInterpolation(Vec<Expression>),
    Spread(Box<Expression>),
    Nothing,
    Garbage,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum RecordItem {
    /// A key: val mapping
    Pair(Expression, Expression),
    /// Span for the "..." and the expression that's being spread
    Spread(Span, Expression),
}
