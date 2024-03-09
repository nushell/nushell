use std::sync::{Arc, Weak};

use nu_protocol::{PluginIdentity, RegisteredPlugin, ShellError, Span};

use super::PersistentPlugin;

#[derive(Debug, Clone)]
pub(crate) struct PluginSource {
    /// The identity of the plugin
    pub(crate) identity: Arc<PluginIdentity>,
    /// A weak reference to the persistent plugin that might hold an interface to the plugin.
    ///
    /// This is weak to avoid cyclic references, but it does mean we might fail to upgrade if
    /// the engine state lost the [`PersistentPlugin`] at some point.
    pub(crate) persistent: Weak<PersistentPlugin>,
}

impl PluginSource {
    /// Create from an `Arc<PersistentPlugin>`
    pub(crate) fn new(plugin: &Arc<PersistentPlugin>) -> PluginSource {
        PluginSource {
            identity: plugin.identity().clone().into(),
            persistent: Arc::downgrade(plugin),
        }
    }

    /// Create a new fake source with a fake identity, for testing
    ///
    /// Warning: [`.persistent()`] will always return an error.
    #[cfg(test)]
    pub(crate) fn new_fake(name: &str) -> PluginSource {
        PluginSource {
            identity: PluginIdentity::new_fake(name).into(),
            persistent: Weak::new(),
        }
    }

    /// Try to upgrade the persistent reference, and return an error referencing `span` as the
    /// object that referenced it otherwise
    pub(crate) fn persistent(
        &self,
        span: Option<Span>,
    ) -> Result<Arc<PersistentPlugin>, ShellError> {
        self.persistent
            .upgrade()
            .ok_or_else(|| ShellError::GenericError {
                error: format!("The `{}` plugin is no longer present", self.identity.name()),
                msg: "removed since this object was created".into(),
                span,
                help: Some("try recreating the object that came from the plugin".into()),
                inner: vec![],
            })
    }

    /// Sources are compatible if their identities are equal
    pub(crate) fn is_compatible(&self, other: &PluginSource) -> bool {
        self.identity == other.identity
    }
}

impl std::ops::Deref for PluginSource {
    type Target = PluginIdentity;

    fn deref(&self) -> &PluginIdentity {
        &self.identity
    }
}
