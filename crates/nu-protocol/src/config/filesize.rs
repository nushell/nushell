use super::prelude::*;
use crate as nu_protocol;

#[derive(Clone, Debug, IntoValue, PartialEq, Eq, Serialize, Deserialize)]
pub struct FilesizeConfig {
    pub metric: bool,
    pub format: String,
}

impl Default for FilesizeConfig {
    fn default() -> Self {
        Self {
            metric: false,
            format: "auto".into(),
        }
    }
}
