use derive_new::new;
use getset::Getters;
use nu_source::Span;
use serde::{Deserialize, Serialize};

#[derive(
    Debug, Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Getters, Serialize, Deserialize, new,
)]
#[get = "pub"]
pub struct ExternalCommand {
    pub(crate) name: Span,
}
