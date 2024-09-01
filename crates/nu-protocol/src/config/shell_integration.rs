use super::prelude::*;
use crate as nu_protocol;

#[derive(Clone, Copy, Debug, IntoValue, PartialEq, Eq, Serialize, Deserialize)]
pub struct ShellIntegrationConfig {
    pub osc2: bool,
    pub osc7: bool,
    pub osc8: bool,
    pub osc9_9: bool,
    pub osc133: bool,
    pub osc633: bool,
    pub reset_application_mode: bool,
}

#[allow(clippy::derivable_impls)]
impl Default for ShellIntegrationConfig {
    fn default() -> Self {
        Self {
            osc2: false,
            osc7: false,
            osc8: false,
            osc9_9: false,
            osc133: false,
            osc633: false,
            reset_application_mode: false,
        }
    }
}
