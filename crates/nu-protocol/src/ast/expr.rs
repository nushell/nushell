use chrono::FixedOffset;
use serde::{Deserialize, Serialize};

use super::{
    Call, CellPath, Expression, ExternalArgument, FullCellPath, MatchPattern, Operator,
    RangeOperator,
};
use crate::{
    ast::ImportPattern, ast::Unit, engine::EngineState, BlockId, OutDest, Signature, Span, Spanned,
    VarId,
};

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
    ExternalCall(Box<Expression>, Vec<ExternalArgument>), // head, args
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
    BarewordInterpolation(Vec<Expression>),
    Spread(Box<Expression>),
    Nothing,
    Garbage,
}

impl Expr {
    pub fn pipe_redirection(
        &self,
        engine_state: &EngineState,
    ) -> (Option<OutDest>, Option<OutDest>) {
        // Usages of `$in` will be wrapped by a `collect` call by the parser,
        // so we do not have to worry about that when considering
        // which of the expressions below may consume pipeline output.
        match self {
            Expr::Call(call) => engine_state.get_decl(call.decl_id).pipe_redirection(),
            Expr::Subexpression(block_id) | Expr::Block(block_id) => engine_state
                .get_block(*block_id)
                .pipe_redirection(engine_state),
            Expr::FullCellPath(cell_path) => cell_path.head.expr.pipe_redirection(engine_state),
            Expr::Bool(_)
            | Expr::Int(_)
            | Expr::Float(_)
            | Expr::Binary(_)
            | Expr::Range(_, _, _, _)
            | Expr::Var(_)
            | Expr::UnaryNot(_)
            | Expr::BinaryOp(_, _, _)
            | Expr::Closure(_) // piping into a closure value, not into a closure call
            | Expr::List(_)
            | Expr::Table(_, _)
            | Expr::Record(_)
            | Expr::ValueWithUnit(_, _)
            | Expr::DateTime(_)
            | Expr::String(_)
            | Expr::CellPath(_)
            | Expr::StringInterpolation(_)
            | Expr::BarewordInterpolation(_)
            | Expr::Nothing => {
                // These expressions do not use the output of the pipeline in any meaningful way,
                // so we can discard the previous output by redirecting it to `Null`.
                (Some(OutDest::Null), None)
            }
            Expr::VarDecl(_)
            | Expr::Operator(_)
            | Expr::Filepath(_, _)
            | Expr::Directory(_, _)
            | Expr::GlobPattern(_, _)
            | Expr::ImportPattern(_)
            | Expr::Overlay(_)
            | Expr::Signature(_)
            | Expr::Spread(_)
            | Expr::Garbage => {
                // These should be impossible to pipe to,
                // but even it is, the pipeline output is not used in any way.
                (Some(OutDest::Null), None)
            }
            Expr::RowCondition(_) | Expr::MatchBlock(_) => {
                // These should be impossible to pipe to,
                // but if they are, then the pipeline output could be used.
                (None, None)
            }
            Expr::ExternalCall(_, _) => {
                // No override necessary, pipes will always be created in eval
                (None, None)
            }
            Expr::Keyword(_, _, _) => {
                // Not sure about this; let's return no redirection override for now.
                (None, None)
            }
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum RecordItem {
    /// A key: val mapping
    Pair(Expression, Expression),
    /// Span for the "..." and the expression that's being spread
    Spread(Span, Expression),
}
