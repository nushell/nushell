use std::{
    collections::{BTreeMap, BTreeSet, HashMap, HashSet},
    sync::{
        Arc, Mutex,
        mpsc::{Receiver, RecvTimeoutError, Sender, TryRecvError},
    },
};

#[cfg(not(target_family = "wasm"))]
use std::time::{Duration, Instant};

use nu_system::{UnfreezeHandle, kill_by_pid};

use crate::{PipelineData, Signals, shell_error};

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
    pub fn kill_and_remove(&mut self, id: JobId) -> shell_error::io::Result<()> {
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
    pub fn kill_all(&mut self) -> shell_error::io::Result<()> {
        self.last_frozen_job_id = None;

        let first_err = self
            .iter()
            .map(|(_, job)| job.kill().err())
            .fold(None, |acc, x| acc.or(x));

        self.jobs.clear();

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
    pub sender: Sender<Mail>,
}

impl ThreadJob {
    pub fn new(signals: Signals, tag: Option<String>, sender: Sender<Mail>) -> Self {
        ThreadJob {
            signals,
            pids: Arc::new(Mutex::new(HashSet::default())),
            sender,
            tag,
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

    pub fn kill(&self) -> shell_error::io::Result<()> {
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
}

impl Job {
    pub fn kill(&self) -> shell_error::io::Result<()> {
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
    pub fn kill(&self) -> shell_error::io::Result<()> {
        #[cfg(unix)]
        {
            Ok(kill_by_pid(self.unfreeze.pid() as i64)?)
        }

        // it doesn't happen outside unix.
        #[cfg(not(unix))]
        {
            Ok(())
        }
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
