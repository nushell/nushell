use crate::{ast::Expression, engine::StateWorkingSet, OutDest, Span, VarId};
use serde::{Deserialize, Serialize};
use std::fmt::Display;

#[derive(Debug, Clone, Copy, Serialize, Deserialize, Eq, PartialEq)]
pub enum RedirectionSource {
    Stdout,
    Stderr,
    StdoutAndStderr,
}

impl Display for RedirectionSource {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(match self {
            RedirectionSource::Stdout => "stdout",
            RedirectionSource::Stderr => "stderr",
            RedirectionSource::StdoutAndStderr => "stdout and stderr",
        })
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum RedirectionTarget {
    File {
        expr: Expression,
        append: bool,
        span: Span,
    },
    Pipe {
        span: Span,
    },
}

impl RedirectionTarget {
    pub fn span(&self) -> Span {
        match self {
            RedirectionTarget::File { span, .. } | RedirectionTarget::Pipe { span } => *span,
        }
    }

    pub fn expr(&self) -> Option<&Expression> {
        match self {
            RedirectionTarget::File { expr, .. } => Some(expr),
            RedirectionTarget::Pipe { .. } => None,
        }
    }

    pub fn has_in_variable(&self, working_set: &StateWorkingSet) -> bool {
        self.expr().is_some_and(|e| e.has_in_variable(working_set))
    }

    pub fn replace_span(
        &mut self,
        working_set: &mut StateWorkingSet,
        replaced: Span,
        new_span: Span,
    ) {
        match self {
            RedirectionTarget::File { expr, .. } => {
                expr.replace_span(working_set, replaced, new_span)
            }
            RedirectionTarget::Pipe { .. } => {}
        }
    }

    pub fn replace_in_variable(
        &mut self,
        working_set: &mut StateWorkingSet<'_>,
        new_var_id: VarId,
    ) {
        match self {
            RedirectionTarget::File { expr, .. } => {
                expr.replace_in_variable(working_set, new_var_id)
            }
            RedirectionTarget::Pipe { .. } => {}
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum PipelineRedirection {
    Single {
        source: RedirectionSource,
        target: RedirectionTarget,
    },
    Separate {
        out: RedirectionTarget,
        err: RedirectionTarget,
    },
}
impl PipelineRedirection {
    pub fn replace_in_variable(
        &mut self,
        working_set: &mut StateWorkingSet<'_>,
        new_var_id: VarId,
    ) {
        match self {
            PipelineRedirection::Single { source: _, target } => {
                target.replace_in_variable(working_set, new_var_id)
            }
            PipelineRedirection::Separate { out, err } => {
                out.replace_in_variable(working_set, new_var_id);
                err.replace_in_variable(working_set, new_var_id);
            }
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PipelineElement {
    pub pipe: Option<Span>,
    pub expr: Expression,
    pub redirection: Option<PipelineRedirection>,
}

impl PipelineElement {
    pub fn has_in_variable(&self, working_set: &StateWorkingSet) -> bool {
        self.expr.has_in_variable(working_set)
            || self.redirection.as_ref().is_some_and(|r| match r {
                PipelineRedirection::Single { target, .. } => target.has_in_variable(working_set),
                PipelineRedirection::Separate { out, err } => {
                    out.has_in_variable(working_set) || err.has_in_variable(working_set)
                }
            })
    }

    pub fn replace_span(
        &mut self,
        working_set: &mut StateWorkingSet,
        replaced: Span,
        new_span: Span,
    ) {
        self.expr.replace_span(working_set, replaced, new_span);
        if let Some(expr) = self.redirection.as_mut() {
            match expr {
                PipelineRedirection::Single { target, .. } => {
                    target.replace_span(working_set, replaced, new_span)
                }
                PipelineRedirection::Separate { out, err } => {
                    out.replace_span(working_set, replaced, new_span);
                    err.replace_span(working_set, replaced, new_span);
                }
            }
        }
    }

    pub fn pipe_redirection(
        &self,
        working_set: &StateWorkingSet,
    ) -> (Option<OutDest>, Option<OutDest>) {
        self.expr.expr.pipe_redirection(working_set)
    }

    pub fn replace_in_variable(
        &mut self,
        working_set: &mut StateWorkingSet<'_>,
        new_var_id: VarId,
    ) {
        self.expr.replace_in_variable(working_set, new_var_id);
        if let Some(redirection) = &mut self.redirection {
            redirection.replace_in_variable(working_set, new_var_id);
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Pipeline {
    pub elements: Vec<PipelineElement>,
}

impl Default for Pipeline {
    fn default() -> Self {
        Self::new()
    }
}

impl Pipeline {
    pub fn new() -> Self {
        Self { elements: vec![] }
    }

    pub fn from_vec(expressions: Vec<Expression>) -> Pipeline {
        Self {
            elements: expressions
                .into_iter()
                .enumerate()
                .map(|(idx, expr)| PipelineElement {
                    pipe: if idx == 0 { None } else { Some(expr.span) },
                    expr,
                    redirection: None,
                })
                .collect(),
        }
    }

    pub fn len(&self) -> usize {
        self.elements.len()
    }

    pub fn is_empty(&self) -> bool {
        self.elements.is_empty()
    }

    pub fn pipe_redirection(
        &self,
        working_set: &StateWorkingSet,
    ) -> (Option<OutDest>, Option<OutDest>) {
        if let Some(first) = self.elements.first() {
            first.pipe_redirection(working_set)
        } else {
            (None, None)
        }
    }
}
