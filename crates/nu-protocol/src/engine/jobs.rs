use std::{
    collections::{BTreeMap, BTreeSet, HashMap, HashSet},
    sync::{
        mpsc::{Receiver, RecvTimeoutError, Sender, TryRecvError},
        Arc, Mutex,
    },
};

#[cfg(not(target_family = "wasm"))]
use std::time::{Duration, Instant};

use nu_system::{kill_by_pid, UnfreezeHandle};

use crate::{PipelineData, Signals, Value};

use crate::JobId;

pub struct Jobs {
    next_job_id: usize,

    // this is the ID of the most recently added frozen job in the jobs table.
    // the methods of this struct must ensure the invariant of this always
    // being None or pointing to a valid job in the table
    last_frozen_job_id: Option<JobId>,
    jobs: HashMap<JobId, Job>,
}

impl Default for Jobs {
    fn default() -> Self {
        Self {
            next_job_id: 1,
            last_frozen_job_id: None,
            jobs: HashMap::default(),
        }
    }
}

impl Jobs {
    pub fn iter(&self) -> impl Iterator<Item = (JobId, &Job)> {
        self.jobs.iter().map(|(k, v)| (*k, v))
    }

    pub fn lookup(&self, id: JobId) -> Option<&Job> {
        self.jobs.get(&id)
    }

    pub fn lookup_mut(&mut self, id: JobId) -> Option<&mut Job> {
        self.jobs.get_mut(&id)
    }

    pub fn remove_job(&mut self, id: JobId) -> Option<Job> {
        if self.last_frozen_job_id.is_some_and(|last| id == last) {
            self.last_frozen_job_id = None;
        }

        self.jobs.remove(&id)
    }

    fn assign_last_frozen_id_if_frozen(&mut self, id: JobId, job: &Job) {
        if let Job::Frozen(_) = job {
            self.last_frozen_job_id = Some(id);
        }
    }

    pub fn add_job(&mut self, job: Job) -> JobId {
        let this_id = JobId::new(self.next_job_id);

        self.assign_last_frozen_id_if_frozen(this_id, &job);

        self.jobs.insert(this_id, job);
        self.next_job_id += 1;

        this_id
    }

    pub fn most_recent_frozen_job_id(&mut self) -> Option<JobId> {
        self.last_frozen_job_id
    }

    // this is useful when you want to remove a job from the list and add it back later
    pub fn add_job_with_id(&mut self, id: JobId, job: Job) -> Result<(), &'static str> {
        self.assign_last_frozen_id_if_frozen(id, &job);

        if let std::collections::hash_map::Entry::Vacant(e) = self.jobs.entry(id) {
            e.insert(job);
            Ok(())
        } else {
            Err("job already exists")
        }
    }

    /// This function tries to forcefully kill a job from this job table,
    /// removes it from the job table. It always succeeds in removing the job
    /// from the table, but may fail in killing the job's active processes.
    pub fn kill_and_remove(&mut self, id: JobId) -> std::io::Result<()> {
        if let Some(job) = self.jobs.get(&id) {
            let err = job.kill();

            self.remove_job(id);

            err?
        }

        Ok(())
    }

    /// This function tries to forcefully kill all the background jobs and
    /// removes all of them from the job table.
    ///
    /// It returns an error if any of the job killing attempts fails, but always
    /// succeeds in removing the jobs from the table.
    pub fn kill_all(&mut self) -> std::io::Result<()> {
        self.last_frozen_job_id = None;

        self.jobs.clear();

        let first_err = self
            .iter()
            .map(|(_, job)| job.kill().err())
            .fold(None, |acc, x| acc.or(x));

        if let Some(err) = first_err {
            Err(err)
        } else {
            Ok(())
        }
    }
}

pub enum Job {
    Thread(ThreadJob),
    Frozen(FrozenJob),
}

// A thread job represents a job that is currently executing as a background thread in nushell.
// This is an Arc-y type, cloning it does not uniquely clone the information of this particular
// job.

// Although rust's documentation does not document the acquire-release semantics of Mutex, this
// is a direct undocumentented requirement of its soundness, and is thus assumed by this
// implementaation.
// see issue https://github.com/rust-lang/rust/issues/126239.
#[derive(Clone)]
pub struct ThreadJob {
    signals: Signals,
    pids: Arc<Mutex<HashSet<u32>>>,
    tag: Option<String>,
    on_termination: Waiter<Value>,
    pub sender: Sender<Mail>,
}

impl ThreadJob {
    pub fn new(
        signals: Signals,
        tag: Option<String>,
        sender: Sender<Mail>,
        on_termination: Waiter<Value>,
    ) -> Self {
        ThreadJob {
            signals,
            pids: Arc::new(Mutex::new(HashSet::default())),
            sender,
            tag,
            on_termination,
        }
    }

    /// Tries to add the provided pid to the active pid set of the current job.
    ///
    /// Returns true if the pid was added successfully, or false if the
    /// current job is interrupted.
    pub fn try_add_pid(&self, pid: u32) -> bool {
        let mut pids = self.pids.lock().expect("PIDs lock was poisoned");

        // note: this signals check must occur after the pids lock has been locked.
        if self.signals.interrupted() {
            false
        } else {
            pids.insert(pid);
            true
        }
    }

    pub fn collect_pids(&self) -> Vec<u32> {
        let lock = self.pids.lock().expect("PID lock was poisoned");

        lock.iter().copied().collect()
    }

    pub fn kill(&self) -> std::io::Result<()> {
        // it's okay to make this interrupt outside of the mutex, since it has acquire-release
        // semantics.

        self.signals.trigger();

        let mut pids = self.pids.lock().expect("PIDs lock was poisoned");

        for pid in pids.iter() {
            kill_by_pid((*pid).into())?;
        }

        pids.clear();

        Ok(())
    }

    pub fn remove_pid(&self, pid: u32) {
        let mut pids = self.pids.lock().expect("PID lock was poisoned");

        pids.remove(&pid);
    }

    pub fn on_termination(&self) -> &Waiter<Value> {
        &self.on_termination
    }
}

impl Job {
    pub fn kill(&self) -> std::io::Result<()> {
        match self {
            Job::Thread(thread_job) => thread_job.kill(),
            Job::Frozen(frozen_job) => frozen_job.kill(),
        }
    }

    pub fn tag(&self) -> Option<&String> {
        match self {
            Job::Thread(thread_job) => thread_job.tag.as_ref(),
            Job::Frozen(frozen_job) => frozen_job.tag.as_ref(),
        }
    }

    pub fn assign_tag(&mut self, tag: Option<String>) {
        match self {
            Job::Thread(thread_job) => thread_job.tag = tag,
            Job::Frozen(frozen_job) => frozen_job.tag = tag,
        }
    }
}

pub struct FrozenJob {
    pub unfreeze: UnfreezeHandle,
    pub tag: Option<String>,
}

impl FrozenJob {
    pub fn kill(&self) -> std::io::Result<()> {
        #[cfg(unix)]
        {
            kill_by_pid(self.unfreeze.pid() as i64)
        }

        // it doesn't happen outside unix.
        #[cfg(not(unix))]
        {
            Ok(())
        }
    }
}

use std::sync::OnceLock;

/// A synchronization primitive that allows multiple threads to wait for a single event to be completed.
///
/// A Waiter/Completer pair is similar to a Receiver/Sender pair from std::sync::mpsc, with a few important differences:
/// - Only one value can only be sent/completed, subsequent completions are ignored
/// - Multiple threads can wait for the completion of an event (`Waiter` is `Clone` unlike `Receiver`)
///
/// This type differs from `OnceLock` only in a few regards:
/// - It is split into `Waiter` and `Completer` halves
/// - It allows users to `wait` on the completion event with a timeout
///
/// Threads that call the [`wait`] method of the `Waiter` block until the [`complete`] method of a matching `Completer` is called.
/// Once [`complete`] is called, all currently waiting threads will be woken up and will return from their `wait` calls.
/// Subsequent calls to [`wait`] will not block and will return immediately.
///
pub fn completion_signal<T>() -> (Completer<T>, Waiter<T>) {
    let inner = Arc::new(InnerWaitCompleteSignal::new());

    (
        Completer {
            inner: inner.clone(),
        },
        Waiter { inner },
    )
}

/// Waiter and Completer are effectively just `Arc` wrappers around this type.
struct InnerWaitCompleteSignal<T> {
    // One may ask: "Why the mutex and the convar"?
    // It turns out OnceLock doesn't have a `wait_timeout` method, so
    // we use the one from the condvar.
    //
    // We once again, assume acquire-release semamntics for Rust mutexes
    mutex: std::sync::Mutex<()>,
    var: std::sync::Condvar,
    value: std::sync::OnceLock<T>,
}

impl<T> InnerWaitCompleteSignal<T> {
    pub fn new() -> Self {
        InnerWaitCompleteSignal {
            mutex: std::sync::Mutex::new(()),
            value: OnceLock::new(),
            var: std::sync::Condvar::new(),
        }
    }
}

#[derive(Clone)]
pub struct Waiter<T> {
    inner: Arc<InnerWaitCompleteSignal<T>>,
}

pub struct Completer<T> {
    inner: Arc<InnerWaitCompleteSignal<T>>,
}

impl<T> Waiter<T> {
    /// Blocks the current thread until a completion signal is sent.
    ///
    /// If the signal has already been emitted, this method returns immediately.
    ///
    pub fn wait(&self) -> &T {
        let inner: &InnerWaitCompleteSignal<T> = self.inner.as_ref();

        let mut guard = inner.mutex.lock().expect("mutex is poisoned!");

        loop {
            match inner.value.get() {
                None => match inner.var.wait(guard) {
                    Ok(it) => guard = it,
                    Err(_) => panic!("mutex is poisoned!"),
                },
                Some(value) => return value,
            }
        }
    }

    pub fn wait_timeout(&self, duration: std::time::Duration) -> Option<&T> {
        let inner: &InnerWaitCompleteSignal<T> = self.inner.as_ref();

        let guard = inner.mutex.lock().expect("mutex is poisoned!");

        match inner
            .var
            .wait_timeout_while(guard, duration, |_| inner.value.get().is_none())
        {
            Ok((_guard, result)) => {
                if result.timed_out() {
                    None
                } else {
                    // SAFETY:
                    // This should never fail, since we just ran a `wait_timeout_while`
                    // that should run while the `inner.value` OnceLock is not defined.
                    // Therefore, it by this point in the code, either a timeout happened,
                    // or a call to the `.get()` method of the OnceLock returned `Some`thing.
                    // A OnceLock cannot be undefined once it is defined, so any subsequent call
                    // to `inner.value.get()` should return `Some`thing.
                    Some(inner.value.get().expect("OnceLock was not defined!"))
                }
            }
            Err(_) => panic!("mutex is poisoned!"),
        }
    }

    // TODO: add wait_timeout

    /// Checks if this completion signal has been signaled.
    ///
    /// This method returns `true` if the [`signal`] method has been called at least once,
    /// and `false` otherwise. This method does not block the current thread.
    ///
    pub fn is_completed(&self) -> bool {
        self.try_get().is_some()
    }

    /// Returns the completed value, or None if none was sent.
    pub fn try_get(&self) -> Option<&T> {
        let _guard = self.inner.mutex.lock().expect("mutex is poisoned!");

        self.inner.value.get()
    }
}

impl<T> Completer<T> {
    /// Signals all threads currently waiting on this completion signal.
    ///
    /// This method sets wakes up all threads that are blocked in the [`wait`] method
    /// of an attached `Waiter`. Subsequent calls to [`wait`] from any thread will return immediately.
    /// This operation has no effect if this completion signal has already been completed.
    pub fn complete(&self, value: T) {
        let inner: &InnerWaitCompleteSignal<T> = self.inner.as_ref();

        let mut _guard = inner.mutex.lock().expect("mutex is poisoned!");

        let _ = inner.value.set(value);

        inner.var.notify_all();
    }
}

/// Stores the information about the background job currently being executed by this thread, if any
#[derive(Clone)]
pub struct CurrentJob {
    pub id: JobId,

    // The background thread job associated with this thread.
    // If None, it indicates this thread is currently the main job
    pub background_thread_job: Option<ThreadJob>,

    // note: although the mailbox is Mutex'd, it is only ever accessed
    // by the current job's threads
    pub mailbox: Arc<Mutex<Mailbox>>,
}

// The storage for unread messages
//
// Messages are initially sent over a mpsc channel,
// and may then be stored in a IgnoredMail struct when
// filtered out by a tag.
pub struct Mailbox {
    receiver: Receiver<Mail>,
    ignored_mail: IgnoredMail,
}

impl Mailbox {
    pub fn new(receiver: Receiver<Mail>) -> Self {
        Mailbox {
            receiver,
            ignored_mail: IgnoredMail::default(),
        }
    }

    #[cfg(not(target_family = "wasm"))]
    pub fn recv_timeout(
        &mut self,
        filter_tag: Option<FilterTag>,
        timeout: Duration,
    ) -> Result<PipelineData, RecvTimeoutError> {
        if let Some(value) = self.ignored_mail.pop(filter_tag) {
            Ok(value)
        } else {
            let mut waited_so_far = Duration::ZERO;
            let mut before = Instant::now();

            while waited_so_far < timeout {
                let (tag, value) = self.receiver.recv_timeout(timeout - waited_so_far)?;

                if filter_tag.is_none() || filter_tag == tag {
                    return Ok(value);
                } else {
                    self.ignored_mail.add((tag, value));
                    let now = Instant::now();
                    waited_so_far += now - before;
                    before = now;
                }
            }

            Err(RecvTimeoutError::Timeout)
        }
    }

    #[cfg(not(target_family = "wasm"))]
    pub fn try_recv(
        &mut self,
        filter_tag: Option<FilterTag>,
    ) -> Result<PipelineData, TryRecvError> {
        if let Some(value) = self.ignored_mail.pop(filter_tag) {
            Ok(value)
        } else {
            loop {
                let (tag, value) = self.receiver.try_recv()?;

                if filter_tag.is_none() || filter_tag == tag {
                    return Ok(value);
                } else {
                    self.ignored_mail.add((tag, value));
                }
            }
        }
    }

    pub fn clear(&mut self) {
        self.ignored_mail.clear();

        while self.receiver.try_recv().is_ok() {}
    }
}

// A data structure used to store messages which were received, but currently ignored by a tag filter
// messages are added and popped in a first-in-first-out matter.
#[derive(Default)]
struct IgnoredMail {
    next_id: usize,
    messages: BTreeMap<usize, Mail>,
    by_tag: HashMap<FilterTag, BTreeSet<usize>>,
}

pub type FilterTag = u64;
pub type Mail = (Option<FilterTag>, PipelineData);

impl IgnoredMail {
    pub fn add(&mut self, (tag, value): Mail) {
        let id = self.next_id;
        self.next_id += 1;

        self.messages.insert(id, (tag, value));

        if let Some(tag) = tag {
            self.by_tag.entry(tag).or_default().insert(id);
        }
    }

    pub fn pop(&mut self, tag: Option<FilterTag>) -> Option<PipelineData> {
        if let Some(tag) = tag {
            self.pop_oldest_with_tag(tag)
        } else {
            self.pop_oldest()
        }
    }

    pub fn clear(&mut self) {
        self.messages.clear();
        self.by_tag.clear();
    }

    fn pop_oldest(&mut self) -> Option<PipelineData> {
        let (id, (tag, value)) = self.messages.pop_first()?;

        if let Some(tag) = tag {
            let needs_cleanup = if let Some(ids) = self.by_tag.get_mut(&tag) {
                ids.remove(&id);
                ids.is_empty()
            } else {
                false
            };

            if needs_cleanup {
                self.by_tag.remove(&tag);
            }
        }

        Some(value)
    }

    fn pop_oldest_with_tag(&mut self, tag: FilterTag) -> Option<PipelineData> {
        let ids = self.by_tag.get_mut(&tag)?;

        let id = ids.pop_first()?;

        if ids.is_empty() {
            self.by_tag.remove(&tag);
        }

        Some(self.messages.remove(&id)?.1)
    }
}

#[cfg(test)]
mod completion_signal_tests {

    use std::{
        sync::mpsc,
        thread::{self, sleep},
        time::Duration,
    };

    use crate::engine::completion_signal;

    fn run_with_timeout<F>(duration: Duration, lambda: F)
    where
        F: FnOnce() + Send + 'static,
    {
        let (send, recv) = std::sync::mpsc::channel();

        let send_ = send.clone();

        thread::spawn(move || {
            lambda();

            let send = send_;

            send.send(true).expect("send failed");
        });

        thread::spawn(move || {
            thread::sleep(duration);

            send.send(false).expect("send failed");
        });

        let ok = recv.recv().expect("recv failed!");

        assert!(ok, "got timeout!");
    }

    #[test]
    fn wait_returns_when_signaled_from_another_thread() {
        run_with_timeout(Duration::from_secs(1), || {
            let (complete, wait) = completion_signal();

            let wait_ = wait.clone();

            thread::spawn(move || {
                sleep(Duration::from_millis(200));
                assert!(!wait_.is_completed());
                complete.complete(123);
            });

            let result = wait.wait();

            assert!(wait.is_completed());

            assert_eq!(*result, 123);
        });
    }

    #[test]
    fn wait_works_from_multiple_threads() {
        run_with_timeout(Duration::from_secs(1), || {
            let (complete, wait) = completion_signal();
            let (send, recv) = mpsc::channel();

            let thread_count = 4;

            for _ in 0..thread_count {
                let wait_ = wait.clone();
                let send_ = send.clone();

                thread::spawn(move || {
                    let value = wait_.wait();
                    send_.send(*value).expect("send failed");
                });
            }

            complete.complete(321);

            for _ in 0..thread_count {
                let result = recv.recv().expect("recv failed");

                assert_eq!(result, 321);
            }
        })
    }

    #[test]
    fn was_signaled_returns_false_when_struct_is_initialized() {
        let (_, wait) = completion_signal::<()>();

        assert!(!wait.is_completed())
    }

    #[test]
    fn was_signaled_returns_true_when_signal_is_called() {
        let (complete, wait) = completion_signal();

        complete.complete(());

        assert!(wait.is_completed())
    }

    #[test]
    fn wait_returns_when_own_thread_signals() {
        run_with_timeout(Duration::from_secs(1), || {
            let (complete, wait) = completion_signal();

            complete.complete(());

            wait.wait();

            assert!(wait.is_completed())
        })
    }
}
