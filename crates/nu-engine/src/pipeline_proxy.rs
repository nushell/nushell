use std::sync::{
    Arc, Mutex,
    atomic::{AtomicBool, Ordering},
    mpsc,
};
use std::time::Duration;

use nu_protocol::{
    PipelineMetadata, Span, Value,
    engine::{FrozenJob, Job, Jobs},
};
use nu_system::{SIGTSTP_FLAG, SuspendState, UnfreezeHandle};

use crate::command_thread::WorkerOutput;

/// Streaming state preserved across a freeze/resume cycle.
///
/// Stored inside [`FrozenJob::pipeline_state`] as `Box<dyn Any + Send>`.
/// On `job unfreeze`, this is downcast back to `FrozenPipelineState` and used
/// to construct a new [`PipelineProxy`].
pub struct FrozenPipelineState {
    /// Present only for Phase 1 freezes (Ctrl+Z before the worker sent any output).
    ///
    /// On unfreeze, `job_unfreeze` polls this to learn whether the eventual result
    /// is `Immediate` (returns it directly) or `Streaming` (creates a proxy).
    /// `None` for mid-stream freezes, where streaming has already started.
    pub output_rx: Option<mpsc::Receiver<WorkerOutput>>,
    pub value_rx: mpsc::Receiver<Value>,
    pub frozen_rx: mpsc::Receiver<()>,
    pub suspend_state: Arc<SuspendState>,
    pub interrupt: Arc<AtomicBool>,
    pub span: Span,
    pub metadata: Option<PipelineMetadata>,
}

/// A lazy iterator that proxies values from a pipeline worker thread with cooperative
/// Ctrl+Z (SIGTSTP) support.
///
/// `PipelineProxy` is the consumer side of the bounded channel between the worker thread
/// and the main thread.  On each 50ms timeout it checks:
///
/// 1. Ctrl+C (`interrupt`): clears channels, resumes worker so it can observe the interrupt.
/// 2. Ctrl+Z (`SIGTSTP_FLAG`): suspends the worker, waits for confirmation, transfers channel
///    handles into a [`FrozenPipelineState`] stored in a new [`FrozenJob`], and returns `None`.
///
/// After freeze the iterator is exhausted.  On `job unfreeze`, a *new* `PipelineProxy` is
/// created from the same channel handles (retrieved from `FrozenPipelineState`).
pub struct PipelineProxy {
    /// `None` after freeze or stream exhaustion.
    value_rx: Option<mpsc::Receiver<Value>>,
    frozen_rx: Option<mpsc::Receiver<()>>,
    suspend_state: Arc<SuspendState>,
    interrupt: Arc<AtomicBool>,
    jobs: Arc<Mutex<Jobs>>,
    is_interactive: bool,
    span: Span,
    metadata: Option<PipelineMetadata>,
}

impl PipelineProxy {
    /// Create a `PipelineProxy` from frozen pipeline state and the engine's job table.
    ///
    /// Used both for fresh streams (after `WorkerOutput::Streaming`) and resumed streams
    /// (after `job unfreeze`).
    pub fn new(state: FrozenPipelineState, jobs: Arc<Mutex<Jobs>>, is_interactive: bool) -> Self {
        PipelineProxy {
            value_rx: Some(state.value_rx),
            frozen_rx: Some(state.frozen_rx),
            suspend_state: state.suspend_state,
            interrupt: state.interrupt,
            jobs,
            is_interactive,
            span: state.span,
            metadata: state.metadata,
        }
    }
}

impl Iterator for PipelineProxy {
    type Item = Value;

    fn next(&mut self) -> Option<Value> {
        loop {
            let result = self
                .value_rx
                .as_ref()?
                .recv_timeout(Duration::from_millis(50));

            match result {
                Ok(value) => return Some(value),

                Err(mpsc::RecvTimeoutError::Disconnected) => {
                    // Worker finished (stream exhausted or interrupted).
                    self.value_rx = None;
                    self.frozen_rx = None;
                    return None;
                }

                Err(mpsc::RecvTimeoutError::Timeout) => {
                    // Check for Ctrl+C (interrupt).
                    if self.interrupt.load(Ordering::Relaxed) {
                        self.suspend_state.resume(); // wake worker if parked
                        self.value_rx = None;
                        self.frozen_rx = None;
                        return None;
                    }

                    // Check for Ctrl+Z (SIGTSTP).
                    if SIGTSTP_FLAG.swap(false, Ordering::SeqCst) {
                        // Tell the worker to cooperatively park.
                        self.suspend_state.suspend();

                        // Wait briefly for the worker to reach a yield point and notify us.
                        {
                            if let Some(frx) = self.frozen_rx.as_ref() {
                                let _ = frx.recv_timeout(Duration::from_millis(500));
                            }
                        }

                        // Take channels out of the proxy — this proxy is now done.
                        let value_rx = self.value_rx.take()?;
                        let frozen_rx = self.frozen_rx.take().expect("frozen_rx must be Some");

                        let frozen_state = FrozenPipelineState {
                            // Mid-stream freeze: streaming already started, no output_rx needed.
                            output_rx: None,
                            value_rx,
                            frozen_rx,
                            suspend_state: self.suspend_state.clone(),
                            interrupt: self.interrupt.clone(),
                            span: self.span,
                            metadata: self.metadata.clone(),
                        };

                        let handle = UnfreezeHandle::Thread {
                            suspend_state: self.suspend_state.clone(),
                            interrupt: self.interrupt.clone(),
                        };

                        let job = Job::Frozen(FrozenJob {
                            unfreeze: handle,
                            description: Some("pipeline".into()),
                            pipeline_state: Some(Box::new(frozen_state)),
                        });

                        let job_id = self.jobs.lock().expect("jobs lock").add_job(job);

                        if self.is_interactive {
                            eprintln!("\nJob {} is frozen", job_id.get());
                        }

                        return None;
                    }
                }
            }
        }
    }
}
