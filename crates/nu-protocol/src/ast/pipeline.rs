use std::ops::{Index, IndexMut};

use crate::ast::Expression;

#[derive(Debug, Clone)]
pub struct Pipe {
    pub redirect_stdout: bool,
    pub redirect_stderr: bool,
}

impl Default for Pipe {
    fn default() -> Self {
        Self::new(true, false)
    }
}

impl Pipe {
    pub fn new(redirect_stdout: bool, redirect_stderr: bool) -> Self {
        Self {
            redirect_stdout,
            redirect_stderr,
        }
    }
}

#[derive(Debug, Clone)]
pub struct PipelineItem {
    pub expression: Expression,
    pub pipe: Option<Pipe>,
}

impl PipelineItem {
    pub fn from_expr(expression: Expression) -> Self {
        Self {
            expression,
            pipe: None,
        }
    }
}

#[derive(Debug, Clone)]
pub struct Pipeline {
    pub items: Vec<PipelineItem>,
}

impl Default for Pipeline {
    fn default() -> Self {
        Self::new()
    }
}

impl Pipeline {
    pub fn new() -> Self {
        Self { items: vec![] }
    }

    pub fn from_expr_vec(expressions: Vec<Expression>) -> Pipeline {
        let mut items = Vec::new();
        for (i, expr) in expressions.iter().enumerate() {
            let mut item = PipelineItem::from_expr(expr.clone());
            if i != expressions.len() - 1 {
                item.pipe = Some(Pipe::default());
            }

            items.push(item);
        }
        Self { items }
    }

    pub fn get_expr(&self, i: usize) -> Option<&Expression> {
        if i >= self.items.len() {
            None
        } else {
            Some(&self.items[i].expression)
        }
    }

    pub fn len(&self) -> usize {
        self.items.len()
    }

    pub fn is_empty(&self) -> bool {
        self.items.is_empty()
    }
}

impl Index<usize> for Pipeline {
    type Output = Expression;

    fn index(&self, index: usize) -> &Self::Output {
        &self.items[index].expression
    }
}

impl IndexMut<usize> for Pipeline {
    fn index_mut(&mut self, index: usize) -> &mut Self::Output {
        &mut self.items[index].expression
    }
}
