use super::prelude::*;
use crate as nu_protocol;

#[derive(Clone, Debug, Default, IntoValue, Serialize, Deserialize)]
pub struct DatetimeFormatConfig {
    pub normal: Option<String>,
    pub table: Option<String>,
}
