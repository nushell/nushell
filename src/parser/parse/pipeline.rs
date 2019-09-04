use crate::parser::CallNode;
use crate::{Span, Tagged};
use derive_new::new;
use getset::Getters;

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, new)]
pub struct Pipeline {
    pub(crate) parts: Vec<PipelineElement>,
    pub(crate) post_ws: Option<Span>,
}

#[derive(Debug, Clone, Eq, PartialEq, Ord, PartialOrd, Getters, new)]
pub struct PipelineElement {
    pub pipe: Option<Span>,
    pub pre_ws: Option<Span>,
    #[get = "pub(crate)"]
    call: Tagged<CallNode>,
    pub post_ws: Option<Span>,
}
