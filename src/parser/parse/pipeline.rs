use crate::parser::CallNode;
use crate::{Span, Tagged};
use derive_new::new;
use getset::Getters;

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, new)]
pub struct Pipeline {
    crate parts: Vec<PipelineElement>,
    crate post_ws: Option<Span>,
}

#[derive(Debug, Clone, Eq, PartialEq, Ord, PartialOrd, Getters, new)]
pub struct PipelineElement {
    pub pre_ws: Option<Span>,
    #[get = "crate"]
    call: Tagged<CallNode>,
    pub post_ws: Option<Span>,
    pub post_pipe: Option<Span>,
}
