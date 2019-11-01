use crate::prelude::*;
use derive_new::new;
use getset::Getters;
use serde::{Deserialize, Serialize};
use std::fmt;

#[derive(
    Debug, Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Getters, Serialize, Deserialize, new,
)]
#[get = "pub(crate)"]
pub struct ExternalCommand {
    pub(crate) name: Span,
}

impl FormatDebug for ExternalCommand {
    fn fmt_debug(&self, f: &mut DebugFormatter, source: &str) -> fmt::Result {
        write!(f, "{}", self.name.slice(source))?;

        Ok(())
    }
}
