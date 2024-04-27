use chrono::FixedOffset;
use serde::{Deserialize, Serialize};

use super::{
    Call, CellPath, Expression, ExternalArgument, FullCellPath, Keyword, MatchPattern, Operator,
    Range, Table, ValueWithUnit,
};
use crate::{ast::ImportPattern, engine::EngineState, BlockId, OutDest, Signature, Span, VarId};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum Expr {
    Bool(bool),
    Int(i64),
    Float(f64),
    Binary(Vec<u8>),
    Range(Box<Range>),
    Var(VarId),
    VarDecl(VarId),
    Call(Box<Call>),
    ExternalCall(Box<Expression>, Box<[ExternalArgument]>), // head, args
    Operator(Operator),
    RowCondition(BlockId),
    UnaryNot(Box<Expression>),
    BinaryOp(Box<Expression>, Box<Expression>, Box<Expression>), //lhs, op, rhs
    Subexpression(BlockId),
    Block(BlockId),
    Closure(BlockId),
    MatchBlock(Vec<(MatchPattern, Expression)>),
    List(Vec<ListItem>),
    Table(Table),
    Record(Vec<RecordItem>),
    Keyword(Box<Keyword>),
    ValueWithUnit(Box<ValueWithUnit>),
    DateTime(chrono::DateTime<FixedOffset>),
    Filepath(String, bool),
    Directory(String, bool),
    GlobPattern(String, bool),
    String(String),
    CellPath(CellPath),
    FullCellPath(Box<FullCellPath>),
    ImportPattern(Box<ImportPattern>),
    Overlay(Option<BlockId>), // block ID of the overlay's origin module
    Signature(Box<Signature>),
    StringInterpolation(Vec<Expression>),
    Nothing,
    Garbage,
}

// This is to document/enforce the size of `Expr` in bytes.
// We should try to avoid increasing the size of `Expr`,
// and PRs that do so will have to change the number below so that it's noted in review.
const _: () = assert!(std::mem::size_of::<Expr>() <= 40);

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
            | Expr::Range(_)
            | Expr::Var(_)
            | Expr::UnaryNot(_)
            | Expr::BinaryOp(_, _, _)
            | Expr::Closure(_) // piping into a closure value, not into a closure call
            | Expr::List(_)
            | Expr::Table(_)
            | Expr::Record(_)
            | Expr::ValueWithUnit(_)
            | Expr::DateTime(_)
            | Expr::String(_)
            | Expr::CellPath(_)
            | Expr::StringInterpolation(_)
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
            Expr::Keyword(_) => {
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

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum ListItem {
    /// A normal expression
    Item(Expression),
    /// Span for the "..." and the expression that's being spread
    Spread(Span, Expression),
}

impl ListItem {
    pub fn expr(&self) -> &Expression {
        let (ListItem::Item(expr) | ListItem::Spread(_, expr)) = self;
        expr
    }

    pub fn expr_mut(&mut self) -> &mut Expression {
        let (ListItem::Item(expr) | ListItem::Spread(_, expr)) = self;
        expr
    }
}
