use crate::{PluginExample, Signature};
use serde::{Deserialize, Serialize};

/// A simple wrapper for Signature that includes examples.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct PluginSignature {
    pub sig: Signature,
    pub examples: Vec<PluginExample>,
}

impl PluginSignature {
    pub fn new(sig: Signature, examples: Vec<PluginExample>) -> Self {
        Self { sig, examples }
    }

    /// Build an internal signature with default help option
    pub fn build(name: impl Into<String>) -> PluginSignature {
        let sig = Signature::new(name.into()).add_help();
        Self::new(sig, vec![])
    }
}
