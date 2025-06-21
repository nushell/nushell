use crate::*;
use std::fmt::Debug;

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
    pub(crate) const fn new(
        marker: &'static (dyn DynExperimentalOptionMarker + Send + Sync),
    ) -> Self {
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
        self.value
            .get()
            .copied()
            .unwrap_or_else(|| match self.marker.stability() {
                Stability::Unstable => false,
                Stability::Stable => false,
                Stability::StableDefault => true,
                Stability::Deprecated => false,
            })
    }
}

impl Debug for ExperimentalOption {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let add_description = f.sign_plus();
        let mut debug_struct = f.debug_struct("ExperimentalOption");
        debug_struct.field("identifier", &self.identifier());
        debug_struct.field("value", &self.value.get());
        debug_struct.field("stability", &self.stability());
        if add_description {
            debug_struct.field("description", &self.description());
        }
        debug_struct.finish()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn assert_identifiers_are_valid() {
        for option in ALL {
            let identifier = option.identifier();
            assert!(!identifier.is_empty());
            
            let mut chars = identifier.chars();
            let first = chars.next().expect("not empty");
            assert!(first.is_alphabetic());
            assert!(first.is_lowercase());

            for char in chars {
                assert!(char.is_alphanumeric());
                if char.is_alphabetic() {
                    assert!(char.is_lowercase());
                }
            }
        }
    }

    #[test]
    fn assert_description_not_empty() {
        for option in ALL {
            assert!(!option.description().is_empty());
        }
    }
}
