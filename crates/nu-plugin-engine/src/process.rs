use std::sync::{Arc, Mutex, MutexGuard, atomic::AtomicU32};

use nu_protocol::{ShellError, Span};
use nu_system::ForegroundGuard;

/// Provides a utility interface for a plugin interface to manage the process the plugin is running
/// in.
#[derive(Debug)]
pub(crate) struct PluginProcess {
    pid: u32,
    mutable: Mutex<MutablePart>,
}

#[derive(Debug)]
struct MutablePart {
    foreground_guard: Option<ForegroundGuard>,
}

impl PluginProcess {
    /// Manage a plugin process.
    pub(crate) fn new(pid: u32) -> PluginProcess {
        PluginProcess {
            pid,
            mutable: Mutex::new(MutablePart {
                foreground_guard: None,
            }),
        }
    }

    /// The process ID of the plugin.
    pub(crate) fn pid(&self) -> u32 {
        self.pid
    }

    fn lock_mutable(&self) -> Result<MutexGuard<'_, MutablePart>, ShellError> {
        self.mutable.lock().map_err(|_| ShellError::NushellFailed {
            msg: "the PluginProcess mutable lock has been poisoned".into(),
        })
    }

    /// Move the plugin process to the foreground. See [`ForegroundGuard::new`].
    ///
    /// This produces an error if the plugin process was already in the foreground.
    ///
    /// Returns `Some()` on Unix with the process group ID if the plugin process will need to join
    /// another process group to be part of the foreground.
    pub(crate) fn enter_foreground(
        &self,
        span: Span,
        pipeline_state: &Arc<(AtomicU32, AtomicU32)>,
    ) -> Result<Option<u32>, ShellError> {
        let pid = self.pid;
        let mut mutable = self.lock_mutable()?;
        if mutable.foreground_guard.is_none() {
            let guard = ForegroundGuard::new(pid, pipeline_state).map_err(|err| {
                ShellError::GenericError {
                    error: "Failed to enter foreground".into(),
                    msg: err.to_string(),
                    span: Some(span),
                    help: None,
                    inner: vec![],
                }
            })?;
            let pgrp = guard.pgrp();
            mutable.foreground_guard = Some(guard);
            Ok(pgrp)
        } else {
            Err(ShellError::GenericError {
                error: "Can't enter foreground".into(),
                msg: "this plugin is already running in the foreground".into(),
                span: Some(span),
                help: Some(
                    "you may be trying to run the command in parallel, or this may be a bug in \
                        the plugin"
                        .into(),
                ),
                inner: vec![],
            })
        }
    }

    /// Move the plugin process out of the foreground. See [`ForegroundGuard`].
    ///
    /// This is a no-op if the plugin process was already in the background.
    pub(crate) fn exit_foreground(&self) -> Result<(), ShellError> {
        let mut mutable = self.lock_mutable()?;
        drop(mutable.foreground_guard.take());
        Ok(())
    }
}
