use super::prelude::*;

#[derive(Clone, Debug, Default, IntoValue, Serialize, Deserialize)]
pub struct DatetimeFormat {
    pub normal: Option<String>,
    pub table: Option<String>,
}
