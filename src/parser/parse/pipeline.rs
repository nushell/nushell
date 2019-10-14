use crate::parser::TokenNode;
use crate::traits::ToDebug;
use crate::{Span, Spanned};
use derive_new::new;
use getset::Getters;
use std::fmt;

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, new)]
pub struct Pipeline {
    pub(crate) parts: Vec<Spanned<PipelineElement>>,
    // pub(crate) post_ws: Option<Tag>,
}

impl ToDebug for Pipeline {
    fn fmt_debug(&self, f: &mut fmt::Formatter, source: &str) -> fmt::Result {
        for part in self.parts.iter() {
            write!(f, "{}", part.debug(source))?;
        }

        Ok(())
    }
}

#[derive(Debug, Clone, Eq, PartialEq, Ord, PartialOrd, Getters, new)]
pub struct PipelineElement {
    pub pipe: Option<Span>,
    pub tokens: Spanned<Vec<TokenNode>>,
}

impl ToDebug for PipelineElement {
    fn fmt_debug(&self, f: &mut fmt::Formatter, source: &str) -> fmt::Result {
        if let Some(pipe) = self.pipe {
            write!(f, "{}", pipe.slice(source))?;
        }

        for token in &self.tokens.item {
            write!(f, "{}", token.debug(source))?;
        }

        Ok(())
    }
}
