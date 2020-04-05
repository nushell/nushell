mod files;
pub mod hir;
mod lite_parse;
mod parse;
mod shapes;
mod signature;

pub use crate::files::Files;
pub use crate::lite_parse::{lite_parse, LitePipeline};
pub use crate::parse::{
    classify_pipeline, garbage, trim_quotes, ClassifiedCommand, ClassifiedPipeline, Commands,
    InternalCommand,
};
pub use crate::shapes::shapes;
pub use crate::signature::{
    ExternalArg, ExternalArgs, ExternalCommand, Signature, SignatureRegistry,
};
