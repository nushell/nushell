use miette::Diagnostic;
use thiserror::Error;

use crate::{JobId, Span};

/// Errors when working working with jobs.
#[derive(Debug, Clone, Copy, PartialEq, Error, Diagnostic)]
pub enum JobError {
    #[error("Job {id} not found")]
    #[diagnostic(
        code(nu::shell::job::not_found),
        help(
            "The operation could not be completed, there is no job currently running with this id"
        )
    )]
    NotFound { span: Span, id: JobId },

    #[error("No frozen job to unfreeze")]
    #[diagnostic(
        code(nu::shell::job::none_to_unfreeze),
        help("There is currently no frozen job to unfreeze")
    )]
    NoneToUnfreeze { span: Span },

    #[error("Job {id} is not frozen")]
    #[diagnostic(
        code(nu::shell::job::cannot_unfreeze),
        help("You tried to unfreeze a job which is not frozen")
    )]
    CannotUnfreeze { span: Span, id: JobId },

    #[error("The job {id} is frozen")]
    #[diagnostic(
        code(nu::shell::job::already_frozen),
        help("This operation cannot be performed because the job is frozen")
    )]
    AlreadyFrozen { span: Span, id: JobId },

    #[error("No message was received in the requested time interval")]
    #[diagnostic(
        code(nu::shell::job::recv_timeout),
        help("No message arrived within the specified time limit")
    )]
    RecvTimeout { span: Span },
}
