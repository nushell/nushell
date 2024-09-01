use super::prelude::*;

#[derive(Clone, Debug, IntoValue, PartialEq, Eq, Serialize, Deserialize)]
pub struct Filesize {
    pub metric: bool,
    pub format: String,
}

impl Default for Filesize {
    fn default() -> Self {
        Self {
            metric: false,
            format: "auto".into(),
        }
    }
}
