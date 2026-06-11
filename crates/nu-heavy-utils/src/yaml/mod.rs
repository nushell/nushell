mod parse;
use nu_protocol::FromValue;
pub use parse::*;

mod serialize;
pub use serialize::*;

#[non_exhaustive]
#[derive(Debug, Clone, Default, FromValue)]
pub enum Spec {
    #[default]
    #[nu_value(rename = "1.1")]
    V1_1,
    
    #[nu_value(rename = "1.2")]
    V1_2,
}
