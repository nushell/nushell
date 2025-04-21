use std::{
    collections::{HashMap, HashSet},
    sync::{Arc, Mutex},
};

use nu_system::{kill_by_pid, UnfreezeHandle};

use crate::{Signals, Value};

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
    on_termination: Arc<WaitSignal<Value>>,
}

impl ThreadJob {
    pub fn new(
        signals: Signals,
        tag: Option<String>,
        on_termination: Arc<WaitSignal<Value>>,
    ) -> Self {
        ThreadJob {
            signals,
            pids: Arc::new(Mutex::new(HashSet::default())),
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

    pub fn on_termination(&self) -> &Arc<WaitSignal<Value>> {
        return &self.on_termination;
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

/// A synchronization primitive that allows multiple threads to wait for a single event to occur.
///
/// Threads that call the [`join`] method will block until the [`signal`] method is called.
/// Once [`signal`] is called, all currently waiting threads will be woken up and will return from their `join` calls.
/// Subsequent calls to [`join`] will not block and will return immediately.
///
/// The [`was_signaled`] method can be used to check if the signal has been emitted without blocking.
pub struct WaitSignal<T> {
    mutex: std::sync::Mutex<bool>,
    value: std::sync::OnceLock<T>,
    var: std::sync::Condvar,
}

impl<T> WaitSignal<T> {
    /// Creates a new `WaitSignal` in an unsignaled state.
    ///
    /// No threads will be woken up initially.
    pub fn new() -> Self {
        WaitSignal {
            mutex: std::sync::Mutex::new(false),
            value: OnceLock::new(),
            var: std::sync::Condvar::new(),
        }
    }

    /// Blocks the current thread until this `WaitSignal` is signaled.
    ///
    /// If the signal has already been emitted, this method returns immediately.
    ///
    /// # Panics
    ///
    /// This method will panic if the underlying mutex is poisoned. This can happen if another
    /// thread holding the mutex panics.
    pub fn join(&self) -> &T {
        let mut guard = self.mutex.lock().expect("mutex is poisoned!");

        while !*guard {
            match self.var.wait(guard) {
                Ok(it) => guard = it,
                Err(_) => panic!("mutex is poisoned!"),
            }
        }

        return self.value.get().unwrap();
    }

    /// Signals all threads currently waiting on this `WaitSignal`.
    ///
    /// This method sets the internal state to "signaled" and wakes up all threads that are blocked
    /// in the [`join`] method. Subsequent calls to [`join`] from any thread will return immediately.
    /// This operation has no effect if the signal has already been emitted.
    pub fn signal(&self, value: T) {
        let mut guard = self.mutex.lock().expect("mutex is poisoned!");

        *guard = true;
        let _ = self.value.set(value);

        self.var.notify_all();
    }

    /// Checks if this `WaitSignal` has been signaled.
    ///
    /// This method returns `true` if the [`signal`] method has been called at least once,
    /// and `false` otherwise. This method does not block the current thread.
    ///
    /// # Panics
    ///
    /// This method will panic if the underlying mutex is poisoned. This can happen if another
    /// thread holding the mutex panics.
    pub fn was_signaled(&self) -> bool {
        let guard = self.mutex.lock().expect("mutex is poisoned!");

        *guard
    }
}

// TODO: move to testing directory
#[cfg(test)]
mod test {

    use std::{
        sync::{mpsc, Arc},
        thread::{self, sleep},
        time::Duration,
    };

    use pretty_assertions::assert_eq;

    use crate::engine::jobs::WaitSignal;

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

        let result = recv.recv().expect("recv failed!");

        assert!(result == true, "timeout!");
    }

    #[test]
    fn join_returns_when_signaled_from_another_thread() {
        run_with_timeout(Duration::from_secs(1), || {
            let signal = Arc::new(WaitSignal::new());

            let thread_signal = signal.clone();

            thread::spawn(move || {
                sleep(Duration::from_millis(200));
                assert!(!thread_signal.was_signaled());
                thread_signal.signal(123);
            });

            let result = signal.join();

            assert!(signal.was_signaled());

            assert_eq!(*result, 123);
        });
    }

    #[test]
    fn join_works_from_multiple_threads() {
        run_with_timeout(Duration::from_secs(1), || {
            let signal = Arc::new(WaitSignal::new());

            let (send, recv) = mpsc::channel();

            let thread_count = 4;

            for _ in 0..thread_count {
                let signal_ = signal.clone();
                let send_ = send.clone();

                thread::spawn(move || {
                    let value = signal_.join();
                    send_.send(*value).expect("send failed");
                });
            }

            signal.signal(321);

            for _ in 0..thread_count {
                let result = recv.recv().expect("recv failed");

                assert_eq!(result, 321);
            }
        })
    }

    #[test]
    fn was_signaled_returns_false_when_struct_is_initalized() {
        let signal = Arc::new(WaitSignal::<()>::new());

        assert!(!signal.was_signaled())
    }

    #[test]
    fn was_signaled_returns_true_when_signal_is_called() {
        let signal = Arc::new(WaitSignal::new());

        signal.signal(());

        assert!(signal.was_signaled())
    }

    #[test]
    fn join_returns_when_own_thread_signals() {
        run_with_timeout(Duration::from_secs(1), || {
            let signal = Arc::new(WaitSignal::new());

            signal.signal(());

            signal.join();

            assert!(signal.was_signaled())
        })
    }
}
