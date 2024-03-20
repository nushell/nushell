use crate::Span;
use std::sync::Arc;

/// Unit of cached source code
#[derive(Clone)]
pub struct CachedFile {
    // Use Arcs of slice types for more compact representation (capacity less)
    // Could possibly become an `Arc<PathBuf>`
    /// The file name with which the code is associated (also includes REPL input)
    pub name: Arc<str>,
    /// Source code as raw bytes
    pub content: Arc<[u8]>,
    /// global span coordinates that are covered by this [`CachedFile`]
    pub covered_span: Span,
}
