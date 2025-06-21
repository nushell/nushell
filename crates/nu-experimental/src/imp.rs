use std::fmt::Debug;
use crate::*;

pub(crate) trait DynExperimentalOptionMarker {
    fn identifier(&self) -> &'static str;
    fn description(&self) -> &'static str;
    fn stability(&self) -> Stability;
}

impl<M: options::ExperimentalOptionMarker> DynExperimentalOptionMarker for M {
    fn identifier(&self) -> &'static str {
        M::IDENTIFIER
    }

    fn description(&self) -> &'static str {
        M::DESCRIPTION
    }

    fn stability(&self) -> Stability {
        M::STABILITY
    }
}

impl ExperimentalOption {
    pub(crate) const fn new(marker: &'static (dyn DynExperimentalOptionMarker + Send + Sync)) -> Self {
        Self {
            value: OnceLock::new(),
            marker,
        }
    }

    pub fn identifier(&self) -> &'static str {
        self.marker.identifier()
    }

    pub fn description(&self) -> &'static str {
        self.marker.description()
    }

    pub fn stability(&self) -> Stability {
        self.marker.stability()
    }

    pub fn get(&self) -> bool {
        self.value.get().copied().unwrap_or_else(|| match self.marker.stability() {
            Stability::Unstable => false,
            Stability::Stable => false,
            Stability::StableDefault => true,
            Stability::Deprecated => false,
        })
    }
}
