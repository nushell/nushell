use std::{
    collections::{HashMap, HashSet},
    sync::{Arc, Mutex},
};

use nu_system::{kill_by_pid, UnfreezeHandle};

use crate::Signals;

use super::JobId;

#[derive(Default)]
pub struct Jobs {
    next_job_id: JobId,
    last_frozen_job_id: Option<JobId>,
    jobs: HashMap<JobId, Job>,
}

impl Jobs {
    pub fn iter(&self) -> impl Iterator<Item = (JobId, &Job)> {
        self.jobs.iter().map(|(k, v)| (*k, v))
    }

    pub fn lookup(&self, id: JobId) -> Option<&Job> {
        self.jobs.get(&id)
    }

    pub fn remove_job(&mut self, id: JobId) -> Option<Job> {
        self.jobs.remove(&id)
    }

    fn assign_last_frozen_id_if_frozen(&mut self, id: JobId, job: &Job) {
        if let Job::Frozen(_) = job {
            self.last_frozen_job_id = Some(id);
        }
    }

    pub fn add_job(&mut self, job: Job) -> JobId {
        let this_id = self.next_job_id;

        self.assign_last_frozen_id_if_frozen(this_id, &job);

        self.jobs.insert(this_id, job);
        self.next_job_id += 1;

        this_id
    }

    pub fn most_recent_frozen_job_id(&self) -> Option<JobId> {
        let id = self.last_frozen_job_id?;

        if self.jobs.contains_key(&id) {
            Some(id)
        } else {
            None
        }
    }

    // this is useful when you want to remove a job form the list and add it back later
    pub fn add_job_with_id(&mut self, id: JobId, job: Job) -> Result<(), &'static str> {
        self.assign_last_frozen_id_if_frozen(id, &job);

        if let std::collections::hash_map::Entry::Vacant(e) = self.jobs.entry(id) {
            e.insert(job);
            Ok(())
        } else {
            Err("job already exists")
        }
    }

    pub fn kill_and_remove(&mut self, id: JobId) -> std::io::Result<()> {
        if let Some(job) = self.jobs.get(&id) {
            job.kill()?;

            self.jobs.remove(&id);
        }

        Ok(())
    }

    pub fn kill_all(&mut self) -> std::io::Result<()> {
        for (_, job) in self.iter() {
            job.kill()?;
        }

        self.jobs.clear();

        Ok(())
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
}

impl ThreadJob {
    pub fn new(signals: Signals) -> Self {
        ThreadJob {
            signals,
            pids: Arc::new(Mutex::new(HashSet::default())),
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
}

impl Job {
    pub fn kill(&self) -> std::io::Result<()> {
        match self {
            Job::Thread(thread_job) => thread_job.kill(),
            Job::Frozen(frozen_job) => frozen_job.kill(),
        }
    }
}

pub struct FrozenJob {
    pub unfreeze: UnfreezeHandle,
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
