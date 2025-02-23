use crate::engine::StateWorkingSet;

use super::{
    Block, Expr, Expression, ListItem, MatchPattern, Pattern, PipelineRedirection, RecordItem,
};

/// Result of find_map closure
#[derive(Default)]
pub enum FindMapResult<T> {
    Found(T),
    #[default]
    Continue,
    Stop,
}

/// Trait for traversing the AST
pub trait Traverse {
    /// Generic function that do flat_map on an AST node
    /// concatenates all recursive results on sub-expressions
    ///
    /// # Arguments
    /// * `f` - function that overrides the default behavior
    fn flat_map<'a, T, F>(&'a self, working_set: &'a StateWorkingSet, f: &F) -> Vec<T>
    where
        F: Fn(&'a Expression) -> Option<Vec<T>>;

    /// Generic function that do find_map on an AST node
    /// return the first Some
    ///
    /// # Arguments
    /// * `f` - function that overrides the default behavior
    fn find_map<'a, T, F>(&'a self, working_set: &'a StateWorkingSet, f: &F) -> Option<T>
    where
        F: Fn(&'a Expression) -> FindMapResult<T>;
}

impl Traverse for Block {
    fn flat_map<'a, T, F>(&'a self, working_set: &'a StateWorkingSet, f: &F) -> Vec<T>
    where
        F: Fn(&'a Expression) -> Option<Vec<T>>,
    {
        self.pipelines
            .iter()
            .flat_map(|pipeline| {
                pipeline.elements.iter().flat_map(|element| {
                    element.expr.flat_map(working_set, f).into_iter().chain(
                        element
                            .redirection
                            .as_ref()
                            .map(|redir| redir.flat_map(working_set, f))
                            .unwrap_or_default(),
                    )
                })
            })
            .collect()
    }

    fn find_map<'a, T, F>(&'a self, working_set: &'a StateWorkingSet, f: &F) -> Option<T>
    where
        F: Fn(&'a Expression) -> FindMapResult<T>,
    {
        self.pipelines.iter().find_map(|pipeline| {
            pipeline.elements.iter().find_map(|element| {
                element.expr.find_map(working_set, f).or(element
                    .redirection
                    .as_ref()
                    .and_then(|redir| redir.find_map(working_set, f)))
            })
        })
    }
}

impl Traverse for PipelineRedirection {
    fn flat_map<'a, T, F>(&'a self, working_set: &'a StateWorkingSet, f: &F) -> Vec<T>
    where
        F: Fn(&'a Expression) -> Option<Vec<T>>,
    {
        let recur = |expr: &'a Expression| expr.flat_map(working_set, f);
        match self {
            PipelineRedirection::Single { target, .. } => {
                target.expr().map(recur).unwrap_or_default()
            }
            PipelineRedirection::Separate { out, err } => [out, err]
                .iter()
                .filter_map(|t| t.expr())
                .flat_map(recur)
                .collect(),
        }
    }

    fn find_map<'a, T, F>(&'a self, working_set: &'a StateWorkingSet, f: &F) -> Option<T>
    where
        F: Fn(&'a Expression) -> FindMapResult<T>,
    {
        let recur = |expr: &'a Expression| expr.find_map(working_set, f);
        match self {
            PipelineRedirection::Single { target, .. } => {
                target.expr().map(recur).unwrap_or_default()
            }
            PipelineRedirection::Separate { out, err } => {
                [out, err].iter().filter_map(|t| t.expr()).find_map(recur)
            }
        }
    }
}

impl Traverse for Expression {
    fn flat_map<'a, T, F>(&'a self, working_set: &'a StateWorkingSet, f: &F) -> Vec<T>
    where
        F: Fn(&'a Expression) -> Option<Vec<T>>,
    {
        // behavior overridden by f
        if let Some(vec) = f(self) {
            return vec;
        }
        let recur = |expr: &'a Expression| expr.flat_map(working_set, f);
        match &self.expr {
            Expr::RowCondition(block_id)
            | Expr::Subexpression(block_id)
            | Expr::Block(block_id)
            | Expr::Closure(block_id) => {
                let block = working_set.get_block(block_id.to_owned());
                block.flat_map(working_set, f)
            }
            Expr::Range(range) => [&range.from, &range.next, &range.to]
                .iter()
                .filter_map(|e| e.as_ref())
                .flat_map(recur)
                .collect(),
            Expr::Call(call) => call
                .arguments
                .iter()
                .filter_map(|arg| arg.expr())
                .flat_map(recur)
                .collect(),
            Expr::ExternalCall(head, args) => recur(head.as_ref())
                .into_iter()
                .chain(args.iter().flat_map(|arg| recur(arg.expr())))
                .collect(),
            Expr::UnaryNot(expr) | Expr::Collect(_, expr) => recur(expr.as_ref()),
            Expr::BinaryOp(lhs, op, rhs) => recur(lhs)
                .into_iter()
                .chain(recur(op))
                .chain(recur(rhs))
                .collect(),
            Expr::MatchBlock(matches) => matches
                .iter()
                .flat_map(|(pattern, expr)| {
                    pattern
                        .flat_map(working_set, f)
                        .into_iter()
                        .chain(recur(expr))
                })
                .collect(),
            Expr::List(items) => items
                .iter()
                .flat_map(|item| match item {
                    ListItem::Item(expr) | ListItem::Spread(_, expr) => recur(expr),
                })
                .collect(),
            Expr::Record(items) => items
                .iter()
                .flat_map(|item| match item {
                    RecordItem::Spread(_, expr) => recur(expr),
                    RecordItem::Pair(key, val) => [key, val].into_iter().flat_map(recur).collect(),
                })
                .collect(),
            Expr::Table(table) => table
                .columns
                .iter()
                .flat_map(recur)
                .chain(table.rows.iter().flat_map(|row| row.iter().flat_map(recur)))
                .collect(),
            Expr::ValueWithUnit(vu) => recur(&vu.expr),
            Expr::FullCellPath(fcp) => recur(&fcp.head),
            Expr::Keyword(kw) => recur(&kw.expr),
            Expr::StringInterpolation(vec) | Expr::GlobInterpolation(vec, _) => {
                vec.iter().flat_map(recur).collect()
            }
            Expr::AttributeBlock(ab) => ab
                .attributes
                .iter()
                .flat_map(|attr| recur(&attr.expr))
                .chain(recur(&ab.item))
                .collect(),

            _ => Vec::new(),
        }
    }

    fn find_map<'a, T, F>(&'a self, working_set: &'a StateWorkingSet, f: &F) -> Option<T>
    where
        F: Fn(&'a Expression) -> FindMapResult<T>,
    {
        // behavior overridden by f
        match f(self) {
            FindMapResult::Found(t) => Some(t),
            FindMapResult::Stop => None,
            FindMapResult::Continue => {
                let recur = |expr: &'a Expression| expr.find_map(working_set, f);
                match &self.expr {
                    Expr::RowCondition(block_id)
                    | Expr::Subexpression(block_id)
                    | Expr::Block(block_id)
                    | Expr::Closure(block_id) => {
                        let block = working_set.get_block(block_id.to_owned());
                        block.find_map(working_set, f)
                    }
                    Expr::Range(range) => [&range.from, &range.next, &range.to]
                        .iter()
                        .find_map(|e| e.as_ref().and_then(recur)),
                    Expr::Call(call) => call
                        .arguments
                        .iter()
                        .find_map(|arg| arg.expr().and_then(recur)),
                    Expr::ExternalCall(head, args) => {
                        recur(head.as_ref()).or(args.iter().find_map(|arg| recur(arg.expr())))
                    }
                    Expr::UnaryNot(expr) | Expr::Collect(_, expr) => recur(expr.as_ref()),
                    Expr::BinaryOp(lhs, op, rhs) => recur(lhs).or(recur(op)).or(recur(rhs)),
                    Expr::MatchBlock(matches) => matches.iter().find_map(|(pattern, expr)| {
                        pattern.find_map(working_set, f).or(recur(expr))
                    }),
                    Expr::List(items) => items.iter().find_map(|item| match item {
                        ListItem::Item(expr) | ListItem::Spread(_, expr) => recur(expr),
                    }),
                    Expr::Record(items) => items.iter().find_map(|item| match item {
                        RecordItem::Spread(_, expr) => recur(expr),
                        RecordItem::Pair(key, val) => [key, val].into_iter().find_map(recur),
                    }),
                    Expr::Table(table) => table
                        .columns
                        .iter()
                        .find_map(recur)
                        .or(table.rows.iter().find_map(|row| row.iter().find_map(recur))),
                    Expr::ValueWithUnit(vu) => recur(&vu.expr),
                    Expr::FullCellPath(fcp) => recur(&fcp.head),
                    Expr::Keyword(kw) => recur(&kw.expr),
                    Expr::StringInterpolation(vec) | Expr::GlobInterpolation(vec, _) => {
                        vec.iter().find_map(recur)
                    }
                    Expr::AttributeBlock(ab) => ab
                        .attributes
                        .iter()
                        .find_map(|attr| recur(&attr.expr))
                        .or_else(|| recur(&ab.item)),

                    _ => None,
                }
            }
        }
    }
}

impl Traverse for MatchPattern {
    fn flat_map<'a, T, F>(&'a self, working_set: &'a StateWorkingSet, f: &F) -> Vec<T>
    where
        F: Fn(&'a Expression) -> Option<Vec<T>>,
    {
        let recur = |expr: &'a Expression| expr.flat_map(working_set, f);
        let recur_pattern = |pattern: &'a MatchPattern| pattern.flat_map(working_set, f);
        match &self.pattern {
            Pattern::Expression(expr) => recur(expr),
            Pattern::List(patterns) | Pattern::Or(patterns) => {
                patterns.iter().flat_map(recur_pattern).collect()
            }
            Pattern::Record(entries) => {
                entries.iter().flat_map(|(_, p)| recur_pattern(p)).collect()
            }
            _ => Vec::new(),
        }
        .into_iter()
        .chain(self.guard.as_ref().map(|g| recur(g)).unwrap_or_default())
        .collect()
    }

    fn find_map<'a, T, F>(&'a self, working_set: &'a StateWorkingSet, f: &F) -> Option<T>
    where
        F: Fn(&'a Expression) -> FindMapResult<T>,
    {
        let recur = |expr: &'a Expression| expr.find_map(working_set, f);
        let recur_pattern = |pattern: &'a MatchPattern| pattern.find_map(working_set, f);
        match &self.pattern {
            Pattern::Expression(expr) => recur(expr),
            Pattern::List(patterns) | Pattern::Or(patterns) => {
                patterns.iter().find_map(recur_pattern)
            }
            Pattern::Record(entries) => entries.iter().find_map(|(_, p)| recur_pattern(p)),
            _ => None,
        }
        .or(self.guard.as_ref().and_then(|g| recur(g)))
    }
}
