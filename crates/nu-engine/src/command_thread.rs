use std::sync::{
    Arc,
    atomic::{AtomicBool, Ordering},
    mpsc,
};
use std::thread::{self, JoinHandle};
use std::time::Duration;

use nu_protocol::{
    PipelineData, PipelineMetadata, ShellError, Signals, Span, Value,
    ast::Block,
    debugger::WithoutDebug,
    engine::{EngineState, Stack},
};
use nu_system::SuspendState;

use crate::eval_ir::eval_ir_block;
use crate::pipeline_proxy::FrozenPipelineState;

thread_local! {
    static IS_COMMAND_THREAD: std::cell::Cell<bool> = const { std::cell::Cell::new(false) };
}

/// Returns `true` when called from a pipeline worker thread spawned by [`CommandThread`].
///
/// Used by `should_use_threaded_pipeline` to prevent recursive threading.
pub fn is_on_command_thread() -> bool {
    IS_COMMAND_THREAD.with(|c| c.get())
}

/// The first (and only) message the worker sends on `output_rx`.
pub enum WorkerOutput {
    /// A non-streaming result (value, empty, byte-stream, or error).
    Immediate(Result<PipelineData, ShellError>),
    /// A list stream has started.  Individual values arrive on `value_rx`.
    Streaming {
        span: Span,
        metadata: Option<PipelineMetadata>,
    },
}

/// Manages a pipeline worker thread that can be cooperatively suspended (frozen) and resumed.
///
/// The worker evaluates the block and then either:
/// - Sends `WorkerOutput::Immediate` for non-streaming results, or
/// - Sends `WorkerOutput::Streaming` and drains the `ListStream` into a bounded channel.
pub struct CommandThread {
    /// Receives `Immediate` (whole result) or `Streaming` (list-stream started).
    pub output_rx: mpsc::Receiver<WorkerOutput>,
    /// Receives individual `Value`s when the worker is draining a `ListStream`.
    pub value_rx: mpsc::Receiver<Value>,
    /// Receives `()` when the worker thread parks on the suspend condvar.
    pub frozen_rx: mpsc::Receiver<()>,
    pub suspend_state: Arc<SuspendState>,
    pub interrupt: Arc<AtomicBool>,
    // Kept alive so the thread isn't detached; dropped when CommandThread drops.
    _join_handle: JoinHandle<()>,
}

impl CommandThread {
    /// Spawns a pipeline worker thread that evaluates `block` with `input`.
    ///
    /// The spawned thread shares the caller's interrupt `Arc<AtomicBool>` so Ctrl+C reaches it.
    /// A fresh `SuspendState` enables cooperative Ctrl+Z suspension.
    pub fn spawn(
        engine_state: &EngineState,
        stack: Stack,
        block: Arc<Block>,
        input: PipelineData,
    ) -> Self {
        let suspend_state = Arc::new(SuspendState::new());
        let (frozen_tx, frozen_rx) = mpsc::sync_channel::<()>(1);
        suspend_state.set_frozen_notifier(frozen_tx);

        // Share the main interrupt so Ctrl+C propagates into the pipeline thread.
        let interrupt = engine_state
            .signals()
            .interrupt_arc()
            .unwrap_or_else(|| Arc::new(AtomicBool::new(false)));

        let signals = Signals::with_suspend(interrupt.clone(), suspend_state.clone());

        let mut threaded_engine_state = engine_state.clone();
        threaded_engine_state.set_signals(signals);

        // Unbounded: a single message (Immediate or Streaming).
        let (output_tx, output_rx) = mpsc::channel::<WorkerOutput>();
        // Bounded: provides natural backpressure so the worker doesn't race ahead.
        let (value_tx, value_rx) = mpsc::sync_channel::<Value>(256);

        let interrupt_worker = interrupt.clone();
        let suspend_state_worker = suspend_state.clone();

        let join_handle = thread::Builder::new()
            .name("pipeline-worker".into())
            .spawn(move || {
                IS_COMMAND_THREAD.with(|c| c.set(true));

                let result = eval_ir_block::<WithoutDebug>(
                    &threaded_engine_state,
                    &mut { stack },
                    &block,
                    input,
                );

                // Unwrap PipelineExecutionData to PipelineData (exit status is intentionally
                // ignored here — external-command exit codes are handled by the REPL).
                let result: Result<PipelineData, ShellError> = match result {
                    Err(ShellError::Exit { code }) => std::process::exit(code),
                    Ok(ped) => Ok(ped.body),
                    Err(e) => Err(e),
                };

                match result {
                    // Drain a ListStream into the bounded channel so the orchestrator / proxy
                    // can observe SIGTSTP on every element without losing output.
                    Ok(PipelineData::ListStream(stream, metadata)) => {
                        let span = stream.span();
                        let _ = output_tx.send(WorkerOutput::Streaming { span, metadata });

                        for value in stream {
                            // Retry loop: handle backpressure + cooperative suspend.
                            let mut v = value;
                            loop {
                                match value_tx.try_send(v) {
                                    Ok(()) => break,
                                    Err(mpsc::TrySendError::Full(returned)) => {
                                        v = returned;
                                        // Block at a yield point if suspended (Ctrl+Z).
                                        suspend_state_worker.wait_if_suspended();
                                        if interrupt_worker.load(Ordering::Relaxed) {
                                            return;
                                        }
                                        // Brief yield to avoid spinning when the channel is full.
                                        thread::sleep(Duration::from_millis(1));
                                    }
                                    Err(mpsc::TrySendError::Disconnected(_)) => {
                                        // Receiver was dropped (job killed or proxy done).
                                        return;
                                    }
                                }
                            }
                            if interrupt_worker.load(Ordering::Relaxed) {
                                break;
                            }
                        }
                    }

                    // All other result types are sent as a single Immediate message.
                    other => {
                        let _ = output_tx.send(WorkerOutput::Immediate(other));
                    }
                }
            })
            .expect("failed to spawn pipeline worker thread");

        CommandThread {
            output_rx,
            value_rx,
            frozen_rx,
            suspend_state,
            interrupt,
            _join_handle: join_handle,
        }
    }

    /// Consume this `CommandThread` into a [`FrozenPipelineState`].
    ///
    /// `output_rx` is preserved so that `job_unfreeze` can receive the worker's eventual
    /// result message.  For Phase 1 freezes (Ctrl+Z before any output), `output_rx` still
    /// has its message pending; for Streaming-path callers, `output_rx` is already empty
    /// (message already consumed) but including it is harmless.
    ///
    /// The worker thread is detached but exits naturally when channels disconnect or
    /// the interrupt flag is set.
    pub fn into_frozen_state(
        self,
        span: Span,
        metadata: Option<PipelineMetadata>,
    ) -> FrozenPipelineState {
        FrozenPipelineState {
            output_rx: Some(self.output_rx),
            value_rx: self.value_rx,
            frozen_rx: self.frozen_rx,
            suspend_state: self.suspend_state,
            interrupt: self.interrupt,
            span,
            metadata,
        }
    }

    /// Ask the pipeline to cooperatively park at the next yield point.
    pub fn suspend(&self) {
        self.suspend_state.suspend();
    }

    /// Wait up to `timeout` for the pipeline thread to park.
    ///
    /// Returns `true` if the thread confirmed it is parked within the timeout.
    pub fn wait_for_frozen(&self, timeout: Duration) -> bool {
        self.frozen_rx.recv_timeout(timeout).is_ok()
    }
}
