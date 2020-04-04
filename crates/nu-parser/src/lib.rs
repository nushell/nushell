mod errors;
pub mod hir;
mod lite_parse;
mod parse;
mod shapes;
mod signature;

pub use crate::errors::ParseError;
pub use crate::lite_parse::lite_parse;
pub use crate::parse::{
    classify_pipeline, garbage, ClassifiedCommand, ClassifiedPipeline, Commands, InternalCommand,
};
pub use crate::shapes::shapes;
pub use crate::signature::{
    ExternalArg, ExternalArgs, ExternalCommand, Signature, SignatureRegistry,
};
