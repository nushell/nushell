use crate::PersistentPlugin;
use nu_protocol::{PluginGcConfig, RegisteredPlugin};
use std::{
    sync::{mpsc, Arc, Weak},
    thread,
    time::{Duration, Instant},
};

/// Plugin garbage collector
///
/// Many users don't want all of their plugins to stay running indefinitely after using them, so
/// this runs a thread that monitors the plugin's usage and stops it automatically if it meets
/// certain conditions of inactivity.
#[derive(Debug, Clone)]
pub struct PluginGc {
    sender: mpsc::Sender<PluginGcMsg>,
}

impl PluginGc {
    /// Start a new plugin garbage collector. Returns an error if the thread failed to spawn.
    pub fn new(
        config: PluginGcConfig,
        plugin: &Arc<PersistentPlugin>,
    ) -> std::io::Result<PluginGc> {
        let (sender, receiver) = mpsc::channel();

        let mut state = PluginGcState {
            config,
            last_update: None,
            locks: 0,
            disabled: false,
            plugin: Arc::downgrade(plugin),
            name: plugin.identity().name().to_owned(),
        };

        thread::Builder::new()
            .name(format!("plugin gc ({})", plugin.identity().name()))
            .spawn(move || state.run(receiver))?;

        Ok(PluginGc { sender })
    }

    /// Update the garbage collector config
    pub fn set_config(&self, config: PluginGcConfig) {
        let _ = self.sender.send(PluginGcMsg::SetConfig(config));
    }

    /// Ensure all GC messages have been processed
    pub fn flush(&self) {
        let (tx, rx) = mpsc::channel();
        let _ = self.sender.send(PluginGcMsg::Flush(tx));
        // This will block until the channel is dropped, which could be because the send failed, or
        // because the GC got the message
        let _ = rx.recv();
    }

    /// Increment the number of locks held by the plugin
    pub fn increment_locks(&self, amount: i64) {
        let _ = self.sender.send(PluginGcMsg::AddLocks(amount));
    }

    /// Decrement the number of locks held by the plugin
    pub fn decrement_locks(&self, amount: i64) {
        let _ = self.sender.send(PluginGcMsg::AddLocks(-amount));
    }

    /// Set whether the GC is disabled by explicit request from the plugin. This is separate from
    /// the `enabled` option in the config, and overrides that option.
    pub fn set_disabled(&self, disabled: bool) {
        let _ = self.sender.send(PluginGcMsg::SetDisabled(disabled));
    }

    /// Tell the GC to stop tracking the plugin. The plugin will not be stopped. The GC cannot be
    /// reactivated after this request - a new one must be created instead.
    pub fn stop_tracking(&self) {
        let _ = self.sender.send(PluginGcMsg::StopTracking);
    }

    /// Tell the GC that the plugin exited so that it can remove it from the persistent plugin.
    ///
    /// The reason the plugin tells the GC rather than just stopping itself via `source` is that
    /// it can't guarantee that the plugin currently pointed to by `source` is itself, but if the
    /// GC is still running, it hasn't received [`.stop_tracking()`] yet, which means it should be
    /// the right plugin.
    pub fn exited(&self) {
        let _ = self.sender.send(PluginGcMsg::Exited);
    }
}

#[derive(Debug)]
enum PluginGcMsg {
    SetConfig(PluginGcConfig),
    Flush(mpsc::Sender<()>),
    AddLocks(i64),
    SetDisabled(bool),
    StopTracking,
    Exited,
}

#[derive(Debug)]
struct PluginGcState {
    config: PluginGcConfig,
    last_update: Option<Instant>,
    locks: i64,
    disabled: bool,
    plugin: Weak<PersistentPlugin>,
    name: String,
}

impl PluginGcState {
    fn next_timeout(&self, now: Instant) -> Option<Duration> {
        if self.locks <= 0 && !self.disabled {
            self.last_update
                .zip(self.config.enabled.then_some(self.config.stop_after))
                .map(|(last_update, stop_after)| {
                    // If configured to stop, and used at some point, calculate the difference
                    let stop_after_duration = Duration::from_nanos(stop_after.max(0) as u64);
                    let duration_since_last_update = now.duration_since(last_update);
                    stop_after_duration.saturating_sub(duration_since_last_update)
                })
        } else {
            // Don't timeout if there are locks set, or disabled
            None
        }
    }

    // returns `Some()` if the GC should not continue to operate, with `true` if it should stop the
    // plugin, or `false` if it should not
    fn handle_message(&mut self, msg: PluginGcMsg) -> Option<bool> {
        match msg {
            PluginGcMsg::SetConfig(config) => {
                self.config = config;
            }
            PluginGcMsg::Flush(sender) => {
                // Rather than sending a message, we just drop the channel, which causes the other
                // side to disconnect equally well
                drop(sender);
            }
            PluginGcMsg::AddLocks(amount) => {
                self.locks += amount;
                if self.locks < 0 {
                    log::warn!(
                        "Plugin GC ({name}) problem: locks count below zero after adding \
                            {amount}: locks={locks}",
                        name = self.name,
                        locks = self.locks,
                    );
                }
                // Any time locks are modified, that counts as activity
                self.last_update = Some(Instant::now());
            }
            PluginGcMsg::SetDisabled(disabled) => {
                self.disabled = disabled;
            }
            PluginGcMsg::StopTracking => {
                // Immediately exit without stopping the plugin
                return Some(false);
            }
            PluginGcMsg::Exited => {
                // Exit and stop the plugin
                return Some(true);
            }
        }
        None
    }

    fn run(&mut self, receiver: mpsc::Receiver<PluginGcMsg>) {
        let mut always_stop = false;

        loop {
            let Some(msg) = (match self.next_timeout(Instant::now()) {
                Some(duration) => receiver.recv_timeout(duration).ok(),
                None => receiver.recv().ok(),
            }) else {
                // If the timeout was reached, or the channel is disconnected, break the loop
                break;
            };

            log::trace!("Plugin GC ({name}) message: {msg:?}", name = self.name);

            if let Some(should_stop) = self.handle_message(msg) {
                // Exit the GC
                if should_stop {
                    // If should_stop = true, attempt to stop the plugin
                    always_stop = true;
                    break;
                } else {
                    // Don't stop the plugin
                    return;
                }
            }
        }

        // Upon exiting the loop, if the timeout reached zero, or we are exiting due to an Exited
        // message, stop the plugin
        if always_stop
            || self
                .next_timeout(Instant::now())
                .is_some_and(|t| t.is_zero())
        {
            // We only hold a weak reference, and it's not an error if we fail to upgrade it -
            // that just means the plugin is definitely stopped anyway.
            if let Some(plugin) = self.plugin.upgrade() {
                let name = &self.name;
                if let Err(err) = plugin.stop() {
                    log::warn!("Plugin `{name}` failed to be stopped by GC: {err}");
                } else {
                    log::debug!("Plugin `{name}` successfully stopped by GC");
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_state() -> PluginGcState {
        PluginGcState {
            config: PluginGcConfig::default(),
            last_update: None,
            locks: 0,
            disabled: false,
            plugin: Weak::new(),
            name: "test".into(),
        }
    }

    #[test]
    fn timeout_configured_as_zero() {
        let now = Instant::now();
        let mut state = test_state();
        state.config.enabled = true;
        state.config.stop_after = 0;
        state.last_update = Some(now);

        assert_eq!(Some(Duration::ZERO), state.next_timeout(now));
    }

    #[test]
    fn timeout_past_deadline() {
        let now = Instant::now();
        let mut state = test_state();
        state.config.enabled = true;
        state.config.stop_after = Duration::from_secs(1).as_nanos() as i64;
        state.last_update = Some(now - Duration::from_secs(2));

        assert_eq!(Some(Duration::ZERO), state.next_timeout(now));
    }

    #[test]
    fn timeout_with_deadline_in_future() {
        let now = Instant::now();
        let mut state = test_state();
        state.config.enabled = true;
        state.config.stop_after = Duration::from_secs(1).as_nanos() as i64;
        state.last_update = Some(now);

        assert_eq!(Some(Duration::from_secs(1)), state.next_timeout(now));
    }

    #[test]
    fn no_timeout_if_disabled_by_config() {
        let now = Instant::now();
        let mut state = test_state();
        state.config.enabled = false;
        state.last_update = Some(now);

        assert_eq!(None, state.next_timeout(now));
    }

    #[test]
    fn no_timeout_if_disabled_by_plugin() {
        let now = Instant::now();
        let mut state = test_state();
        state.config.enabled = true;
        state.disabled = true;
        state.last_update = Some(now);

        assert_eq!(None, state.next_timeout(now));
    }

    #[test]
    fn no_timeout_if_locks_count_over_zero() {
        let now = Instant::now();
        let mut state = test_state();
        state.config.enabled = true;
        state.locks = 1;
        state.last_update = Some(now);

        assert_eq!(None, state.next_timeout(now));
    }

    #[test]
    fn adding_locks_changes_last_update() {
        let mut state = test_state();
        let original_last_update = Some(Instant::now() - Duration::from_secs(1));
        state.last_update = original_last_update;
        state.handle_message(PluginGcMsg::AddLocks(1));
        assert_ne!(original_last_update, state.last_update, "not updated");
    }
}
