mod parse;
pub use parse::*;

mod serialize;
pub use serialize::*;

#[non_exhaustive]
#[derive(Debug, Clone, Default)]
pub enum Spec {
    V1_1,
    #[default]
    V1_2,
}
