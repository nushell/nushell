use std::process::{Command, ExitStatus};

use crate::{JobExitStatus, Jobs};

impl From<ExitStatus> for JobExitStatus {
    fn from(status: ExitStatus) -> Self {
        status
            .code()
            .map_or(JobExitStatus::Unknown, JobExitStatus::Exited)
    }
}

impl Jobs {
    pub(crate) fn platform_pre_spawn(_command: &mut Command, _interactive: bool) {}
}
