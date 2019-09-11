use crate::parser::CallNode;
use crate::traits::ToDebug;
use crate::{Tag, Tagged};
use derive_new::new;
use getset::Getters;
use std::fmt;

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, new)]
pub struct Pipeline {
    pub(crate) parts: Vec<PipelineElement>,
    pub(crate) post_ws: Option<Tag>,
}

impl ToDebug for Pipeline {
    fn fmt_debug(&self, f: &mut fmt::Formatter, source: &str) -> fmt::Result {
        for part in &self.parts {
            write!(f, "{}", part.debug(source))?;
        }

        if let Some(post_ws) = self.post_ws {
            write!(f, "{}", post_ws.slice(source))?
        }

        Ok(())
    }
}

#[derive(Debug, Clone, Eq, PartialEq, Ord, PartialOrd, Getters, new)]
pub struct PipelineElement {
    pub pipe: Option<Tag>,
    pub pre_ws: Option<Tag>,
    #[get = "pub(crate)"]
    call: Tagged<CallNode>,
    pub post_ws: Option<Tag>,
}

impl ToDebug for PipelineElement {
    fn fmt_debug(&self, f: &mut fmt::Formatter, source: &str) -> fmt::Result {
        if let Some(pipe) = self.pipe {
            write!(f, "{}", pipe.slice(source))?;
        }

        if let Some(pre_ws) = self.pre_ws {
            write!(f, "{}", pre_ws.slice(source))?;
        }

        write!(f, "{}", self.call.debug(source))?;

        if let Some(post_ws) = self.post_ws {
            write!(f, "{}", post_ws.slice(source))?;
        }

        Ok(())
    }
}
