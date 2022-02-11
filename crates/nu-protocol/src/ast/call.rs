use super::Expression;
use crate::{DeclId, Span, Spanned};

#[derive(Debug, Clone)]
pub struct Call {
    /// identifier of the declaration to call
    pub decl_id: DeclId,
    pub head: Span,
    pub positional: Vec<Expression>,
    pub named: Vec<(Spanned<String>, Option<Expression>)>,
}

impl Call {
    pub fn new(head: Span) -> Call {
        Self {
            decl_id: 0,
            head,
            positional: vec![],
            named: vec![],
        }
    }

    pub fn has_flag(&self, flag_name: &str) -> bool {
        for name in &self.named {
            if flag_name == name.0.item {
                return true;
            }
        }

        false
    }

    pub fn get_flag_expr(&self, flag_name: &str) -> Option<Expression> {
        for name in &self.named {
            if flag_name == name.0.item {
                return name.1.clone();
            }
        }

        None
    }

    pub fn get_named_arg(&self, flag_name: &str) -> Option<Spanned<String>> {
        for name in &self.named {
            if flag_name == name.0.item {
                return Some(name.0.clone());
            }
        }

        None
    }

    pub fn nth(&self, pos: usize) -> Option<Expression> {
        self.positional.get(pos).cloned()
    }

    pub fn span(&self) -> Span {
        let mut span = self.head;

        for positional in &self.positional {
            if positional.span.end > span.end {
                span.end = positional.span.end;
            }
        }

        for (named, val) in &self.named {
            if named.span.end > span.end {
                span.end = named.span.end;
            }

            if let Some(val) = &val {
                if val.span.end > span.end {
                    span.end = val.span.end;
                }
            }
        }

        span
    }
}
