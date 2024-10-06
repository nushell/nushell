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
    pub(crate) fn value(span: Span) -> Self {
        RedirectModes {
            out: Some(RedirectMode::Value.into_spanned(span)),
            err: None,
        }
    }

    pub(crate) fn caller(span: Span) -> RedirectModes {
        RedirectModes {
            out: Some(RedirectMode::Caller.into_spanned(span)),
            err: Some(RedirectMode::Caller.into_spanned(span)),
        }
    }
}

pub(crate) fn redirection_target_to_mode(
    working_set: &StateWorkingSet,
    builder: &mut BlockBuilder,
    target: &RedirectionTarget,
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
                RedirectModes::value(*redir_span),
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
        RedirectionTarget::Pipe { span } => RedirectMode::Pipe.into_spanned(*span),
    })
}

pub(crate) fn redirect_modes_of_expression(
    working_set: &StateWorkingSet,
    expression: &Expression,
    redir_span: Span,
) -> Result<RedirectModes, CompileError> {
    let (out, err) = expression.expr.pipe_redirection(working_set);
    Ok(RedirectModes {
        out: out
            .map(|r| r.into_spanned(redir_span))
            .map(out_dest_to_redirect_mode)
            .transpose()?,
        err: err
            .map(|r| r.into_spanned(redir_span))
            .map(out_dest_to_redirect_mode)
            .transpose()?,
    })
}

/// Finish the redirection for an expression, writing to and closing files as necessary
pub(crate) fn finish_redirection(
    builder: &mut BlockBuilder,
    modes: RedirectModes,
    out_reg: RegId,
) -> Result<(), CompileError> {
    if let Some(Spanned {
        item: RedirectMode::File { file_num },
        span,
    }) = modes.out
    {
        // If out is a file and err is a pipe, we must not consume the expression result -
        // that is actually the err, in that case.
        if !matches!(
            modes.err,
            Some(Spanned {
                item: RedirectMode::Pipe { .. },
                ..
            })
        ) {
            builder.push(
                Instruction::WriteFile {
                    file_num,
                    src: out_reg,
                }
                .into_spanned(span),
            )?;
            builder.load_empty(out_reg)?;
        }
        builder.push(Instruction::CloseFile { file_num }.into_spanned(span))?;
    }

    match modes.err {
        Some(Spanned {
            item: RedirectMode::File { file_num },
            span,
        }) => {
            // Close the file, unless it's the same as out (in which case it was already closed)
            if !modes.out.is_some_and(|out_mode| match out_mode.item {
                RedirectMode::File {
                    file_num: out_file_num,
                } => file_num == out_file_num,
                _ => false,
            }) {
                builder.push(Instruction::CloseFile { file_num }.into_spanned(span))?;
            }
        }
        Some(Spanned {
            item: RedirectMode::Pipe,
            span,
        }) => {
            builder.push(Instruction::CheckErrRedirected { src: out_reg }.into_spanned(span))?;
        }
        _ => (),
    }

    Ok(())
}

pub(crate) fn out_dest_to_redirect_mode(
    out_dest: Spanned<OutDest>,
) -> Result<Spanned<RedirectMode>, CompileError> {
    let span = out_dest.span;
    out_dest
        .map(|out_dest| match out_dest {
            OutDest::Pipe => Ok(RedirectMode::Pipe),
            OutDest::PipeSeparate => Ok(RedirectMode::PipeSeparate),
            OutDest::Value => Ok(RedirectMode::Value),
            OutDest::Null => Ok(RedirectMode::Null),
            OutDest::Print => Ok(RedirectMode::Print),
            OutDest::Inherit => Err(CompileError::InvalidRedirectMode { span }),
            OutDest::File(_) => Err(CompileError::InvalidRedirectMode { span }),
        })
        .transpose()
}
