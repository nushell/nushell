use nu_protocol::{
    ast::{Expression, RedirectionTarget},
    engine::StateWorkingSet,
    ir::RedirectMode,
    IntoSpanned, OutDest, Span, Spanned,
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
            let path_reg = builder.next_register()?;
            compile_expression(
                working_set,
                builder,
                expr,
                RedirectModes::capture_out(*redir_span),
                None,
                path_reg,
            )?;
            RedirectMode::File {
                path: path_reg,
                append: *append,
            }
            .into_spanned(*redir_span)
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

pub(crate) fn out_dest_to_redirect_mode(out_dest: OutDest) -> Result<RedirectMode, CompileError> {
    match out_dest {
        OutDest::Pipe => Ok(RedirectMode::Pipe),
        OutDest::Capture => Ok(RedirectMode::Capture),
        OutDest::Null => Ok(RedirectMode::Null),
        OutDest::Inherit => Ok(RedirectMode::Inherit),
        OutDest::File(_) => Err(CompileError::InvalidRedirectMode),
    }
}
