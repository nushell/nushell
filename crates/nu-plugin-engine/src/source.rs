use super::GetPlugin;
use nu_protocol::{PluginIdentity, ShellError, Span};
use std::sync::{Arc, Weak};

/// The source of a custom value or plugin command. Includes a weak reference to the persistent
/// plugin so it can be retrieved.
#[derive(Debug, Clone)]
pub struct PluginSource {
    /// The identity of the plugin
    pub(crate) identity: Arc<PluginIdentity>,
    /// A weak reference to the persistent plugin that might hold an interface to the plugin.
    ///
    /// This is weak to avoid cyclic references, but it does mean we might fail to upgrade if
    /// the engine state lost the [`PersistentPlugin`][crate::PersistentPlugin] at some point.
    pub(crate) persistent: Weak<dyn GetPlugin>,
}

impl PluginSource {
    /// Create from an implementation of `GetPlugin`
    pub fn new(plugin: Arc<dyn GetPlugin>) -> PluginSource {
        PluginSource {
            identity: plugin.identity().clone().into(),
            persistent: Arc::downgrade(&plugin),
        }
    }

    /// Create a new fake source with a fake identity, for testing
    ///
    /// Warning: [`.persistent()`](Self::persistent) will always return an error.
    pub fn new_fake(name: &str) -> PluginSource {
        PluginSource {
            identity: PluginIdentity::new_fake(name).into(),
            persistent: Weak::<crate::PersistentPlugin>::new(),
        }
    }

    /// Try to upgrade the persistent reference, and return an error referencing `span` as the
    /// object that referenced it otherwise
    pub fn persistent(&self, span: Option<Span>) -> Result<Arc<dyn GetPlugin>, ShellError> {
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
