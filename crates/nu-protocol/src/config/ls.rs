use super::prelude::*;

#[derive(Clone, Copy, Debug, IntoValue, PartialEq, Eq, Serialize, Deserialize)]
pub struct LsConfig {
    pub use_ls_colors: bool,
    pub clickable_links: bool,
}

impl Default for LsConfig {
    fn default() -> Self {
        Self {
            use_ls_colors: true,
            clickable_links: true,
        }
    }
}
