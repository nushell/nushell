use crate::Span;
use std::sync::Arc;

#[derive(Clone)]
pub struct CachedFile {
    // Could possibly become an `Arc<PathBuf>`
    pub name: Arc<str>,
    // Use Arcs of slice types for more compact representation (capacity less)
    pub content: Arc<[u8]>,
    // TODO: when refactoring `Span` to IDs this needs to remain an interval
    pub covered_span: Span,
}
