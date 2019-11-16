use crate::parser::TokenNode;
use crate::{DebugFormatter, FormatDebug, Span, Spanned, ToDebug};
use derive_new::new;
use getset::Getters;
use itertools::Itertools;
use std::fmt::{self, Write};

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Getters, new)]
pub struct Pipeline {
    #[get = "pub"]
    pub(crate) parts: Vec<Spanned<PipelineElement>>,
}

impl FormatDebug for Pipeline {
    fn fmt_debug(&self, f: &mut DebugFormatter, source: &str) -> fmt::Result {
        f.say_str(
            "pipeline",
            self.parts.iter().map(|p| p.debug(source)).join(" "),
        )
    }
}

#[derive(Debug, Clone, Eq, PartialEq, Ord, PartialOrd, Getters, new)]
pub struct PipelineElement {
    pub pipe: Option<Span>,
    #[get = "pub"]
    pub tokens: Spanned<Vec<TokenNode>>,
}

impl FormatDebug for PipelineElement {
    fn fmt_debug(&self, f: &mut DebugFormatter, source: &str) -> fmt::Result {
        if let Some(pipe) = self.pipe {
            write!(f, "{}", pipe.slice(source))?;
        }

        for token in &self.tokens.item {
            write!(f, "{}", token.debug(source))?;
        }

        Ok(())
    }
}
