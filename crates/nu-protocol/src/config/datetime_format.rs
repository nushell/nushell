use super::prelude::*;

#[derive(Clone, Debug, Default, IntoValue, Serialize, Deserialize)]
pub struct DatetimeFormatConfig {
    pub normal: Option<String>,
    pub table: Option<String>,
}
