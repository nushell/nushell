use super::prelude::*;

#[derive(Clone, Copy, Debug, IntoValue, PartialEq, Eq, Serialize, Deserialize)]
pub struct Rm {
    pub always_trash: bool,
}

#[allow(clippy::derivable_impls)]
impl Default for Rm {
    fn default() -> Self {
        Self {
            always_trash: false,
        }
    }
}
