use crate::IntoValue;
use serde::{Deserialize, Serialize};

use crate as nu_protocol;

#[derive(Clone, Debug, Default, IntoValue, Serialize, Deserialize)]
pub struct DatetimeFormat {
    pub normal: Option<String>,
    pub table: Option<String>,
}
