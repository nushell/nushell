use nu_protocol::{
    ast::{Expression, RedirectionTarget},
    engine::StateWorkingSet,
    ir::{Instruction, RedirectMode},
    IntoSpanned, OutDest, RegId, Span, Spanned,
};

use super::{compile_expression, BlockBuilder, CompileError};

#[derive(Default, Clone)]
pub(crate) struct RedirectModes {
    pub(crate) out: Option<Spanned<RedirectMode>>,
    pub(crate) err: Option<Spanned<RedirectMode>>,
}

impl RedirectModes {
    pub(crate) fn capture_out(span: Span) -> Self {
        RedirectModes {
            out: Some(RedirectMode::Capture.into_spanned(span)),
            err: None,
        }
    }

    pub(crate) fn caller(span: Span) -> RedirectModes {
        RedirectModes {
            out: Some(RedirectMode::Caller.into_spanned(span)),
            err: Some(RedirectMode::Caller.into_spanned(span)),
        }
    }

    pub(crate) fn with_pipe_out(&self, span: Span) -> Self {
        RedirectModes {
            out: Some(RedirectMode::Pipe.into_spanned(span)),
            err: self.err.clone(),
        }
    }

    pub(crate) fn with_capture_out(&self, span: Span) -> Self {
        RedirectModes {
            out: Some(RedirectMode::Capture.into_spanned(span)),
            err: self.err.clone(),
        }
    }
}

pub(crate) fn redirection_target_to_mode(
    working_set: &StateWorkingSet,
    builder: &mut BlockBuilder,
    target: &RedirectionTarget,
    separate: bool,
) -> Result<Spanned<RedirectMode>, CompileError> {
    Ok(match target {
        RedirectionTarget::File {
            expr,
            append,
            span: redir_span,
        } => {
            let file_num = builder.next_file_num()?;
            let path_reg = builder.next_register()?;
            compile_expression(
                working_set,
                builder,
                expr,
                RedirectModes::capture_out(*redir_span),
                None,
                path_reg,
            )?;
            builder.push(
                Instruction::OpenFile {
                    file_num,
                    path: path_reg,
                    append: *append,
                }
                .into_spanned(*redir_span),
            )?;
            RedirectMode::File { file_num }.into_spanned(*redir_span)
        }
        RedirectionTarget::Pipe { span } => (if separate {
            RedirectMode::Capture
        } else {
            RedirectMode::Pipe
        })
        .into_spanned(*span),
    })
}

pub(crate) fn redirect_modes_of_expression(
    working_set: &StateWorkingSet,
    expression: &Expression,
    redir_span: Span,
) -> Result<RedirectModes, CompileError> {
    let (out, err) = expression.expr.pipe_redirection(&working_set);
    Ok(RedirectModes {
        out: out
            .map(|out| out_dest_to_redirect_mode(out))
            .transpose()?
            .map(|mode| mode.into_spanned(redir_span)),
        err: err
            .map(|err| out_dest_to_redirect_mode(err))
            .transpose()?
            .map(|mode| mode.into_spanned(redir_span)),
    })
}

/// Finish the redirection for an expression, writing to and closing files as necessary
pub(crate) fn finish_redirection(
    builder: &mut BlockBuilder,
    modes: RedirectModes,
    out_reg: RegId,
) -> Result<(), CompileError> {
    match modes.out {
        Some(Spanned {
            item: RedirectMode::File { file_num },
            span,
        }) => {
            builder.push(
                Instruction::WriteFile {
                    file_num,
                    src: out_reg,
                }
                .into_spanned(span),
            )?;
            builder.load_empty(out_reg)?;
            builder.push(Instruction::CloseFile { file_num }.into_spanned(span))?;
        }
        _ => (),
    }

    match modes.err {
        Some(Spanned {
            item: RedirectMode::File { file_num },
            span,
        }) => {
            builder.push(Instruction::CloseFile { file_num }.into_spanned(span))?;
        }
        _ => (),
    }

    Ok(())
}

pub(crate) fn out_dest_to_redirect_mode(out_dest: OutDest) -> Result<RedirectMode, CompileError> {
    match out_dest {
        OutDest::Pipe => Ok(RedirectMode::Pipe),
        OutDest::Capture => Ok(RedirectMode::Capture),
        OutDest::Null => Ok(RedirectMode::Null),
        OutDest::Inherit => Ok(RedirectMode::Inherit),
        OutDest::File(_) => Err(CompileError::InvalidRedirectMode),
    }
}
