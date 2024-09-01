use super::prelude::*;
use crate as nu_protocol;

#[derive(Clone, Copy, Debug, IntoValue, PartialEq, Eq, Serialize, Deserialize)]
pub struct RmConfig {
    pub always_trash: bool,
}

#[allow(clippy::derivable_impls)]
impl Default for RmConfig {
    fn default() -> Self {
        Self {
            always_trash: false,
        }
    }
}
