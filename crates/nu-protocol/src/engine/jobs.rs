use std::collections::HashMap;

use nu_system::UnfreezeHandle;

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
}

pub enum Job {
    Thread(ThreadJob),
    Frozen(FrozenJob),
}

pub struct ThreadJob {
    pub signals: Signals,
}

impl ThreadJob {
    pub fn new(signals: Signals) -> Self {
        ThreadJob { signals }
    }
}

pub struct FrozenJob {
    pub unfreeze: UnfreezeHandle,
}
