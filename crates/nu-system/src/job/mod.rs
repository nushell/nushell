use std::{
    fmt::Display,
    io,
    process::Command,
    sync::{Arc, Mutex, MutexGuard},
};

pub type JobId = usize;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum JobStatus {
    Completed,
    // Stopped,
    Running,
}

impl Display for JobStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{}",
            match self {
                JobStatus::Completed => "done",
                // JobStatus::Stopped => "stopped",
                JobStatus::Running => "running",
            }
        )
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum JobExitStatus {
    Exited(i32),
    Signaled { signal: i32, core_dumped: bool },
    Unknown,
}

#[derive(Debug, Clone)]
pub struct JobInfo {
    pub id: JobId,
    pub command: String,
    pub pid: u32,
    pub exit_status: Option<JobExitStatus>,
}

impl JobInfo {
    pub fn status(&self) -> JobStatus {
        if self.exit_status.is_some() {
            JobStatus::Completed
        } else {
            JobStatus::Running
        }
    }
}

#[derive(Debug)]
struct Job {
    id: JobId,
    command: String,
    pid: u32,
    exit_status: Option<JobExitStatus>,
}

impl Job {
    fn status(&self) -> JobStatus {
        if self.exit_status.is_some() {
            JobStatus::Completed
        } else {
            JobStatus::Running
        }
    }

    fn info(&self) -> JobInfo {
        JobInfo {
            id: self.id,
            command: self.command.clone(),
            exit_status: self.exit_status,
            pid: self.pid,
        }
    }
}

#[derive(Debug, Default)]
struct JobState {
    /// All completed and running jobs in ascending order based on JobId.
    ///
    /// Completed jobs are removed once `background_jobs` is called.
    jobs: Vec<Job>,
}

#[derive(Debug, Clone, Default)]
pub struct Jobs(Arc<Mutex<JobState>>);

impl Jobs {
    pub fn new() -> Self {
        Self::default()
    }

    fn state(&self) -> MutexGuard<JobState> {
        self.0.lock().expect("unpoisoned")
    }

    pub fn spawn_background(&self, mut command: Command, interactive: bool) -> io::Result<JobId> {
        Self::platform_pre_spawn(&mut command, interactive);

        let mut child = command.spawn()?;

        let mut state = self.state();
        let job = Job {
            id: state.jobs.last().map(|job| job.id).unwrap_or(0) + 1,
            command: command.get_program().to_string_lossy().to_string(),
            pid: child.id(),
            exit_status: None,
        };
        let id = job.id;

        // Other commands/libraries can spawn processes outside of job control,
        // so we cannot use waitpid(-1) without potentially messing with that.
        // Instead, we spawn a thread to wait on each background job.
        let thread = {
            let jobs = self.clone();
            std::thread::Builder::new().spawn(move || {
                let status = child.wait();
                if let Some(job) = jobs.state().jobs.iter_mut().find(|job| job.id == id) {
                    debug_assert!(
                        job.exit_status.is_none(),
                        "job with id {id} already had its exit status set"
                    );
                    job.exit_status = Some(status.map_or(JobExitStatus::Unknown, Into::into));
                } else {
                    debug_assert!(false, "did not find job with id {id}")
                }
            })
        };

        if let Err(err) = thread {
            // TODO: the thread failed to spawn, so the child process is not being waited on.
            // On unix, this will leave the child as a zombie process until nushell exits.
            Err(err)
        } else {
            // At this point, the job has succesfully launched, so we can add it.
            state.jobs.push(job);
            Ok(id)
        }
    }

    /// Returns information about each job (runnning and completed).
    /// Note that any completed jobs are removed from the job list.
    pub fn info(&self) -> Vec<JobInfo> {
        let mut state = self.state();
        let jobs = state.jobs.iter().map(Job::info).collect();
        state
            .jobs
            .retain(|job| job.status() != JobStatus::Completed);
        jobs
    }
}

#[cfg(unix)]
mod unix;
#[cfg(unix)]
pub use unix::*;

#[cfg(not(unix))]
mod non_unix;
#[cfg(not(unix))]
pub use non_unix::*;

mod foreground;
pub use foreground::*;
