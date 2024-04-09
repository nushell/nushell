use crate::{ErrSpan, IntoSpanned, ShellError, Span};
use std::{
    io::{ErrorKind, Read, Write},
    sync::atomic::{AtomicBool, Ordering},
};

const DEFAULT_BUF_SIZE: usize = 8192;

pub fn copy_with_interrupt<R: ?Sized, W: ?Sized>(
    reader: &mut R,
    writer: &mut W,
    span: Span,
    interrupt: Option<&AtomicBool>,
) -> Result<u64, ShellError>
where
    R: Read,
    W: Write,
{
    if let Some(interrupt) = interrupt {
        // #[cfg(any(target_os = "linux", target_os = "android"))]
        // {
        //     return crate::sys::kernel_copy::copy_spec(reader, writer);
        // }
        match generic_copy(reader, writer, span, interrupt) {
            Ok(len) => {
                writer.flush().err_span(span)?;
                Ok(len)
            }
            Err(err) => {
                let _ = writer.flush();
                Err(err)
            }
        }
    } else {
        match std::io::copy(reader, writer) {
            Ok(n) => {
                writer.flush().err_span(span)?;
                Ok(n)
            }
            Err(err) => {
                let _ = writer.flush();
                Err(err.into_spanned(span).into())
            }
        }
    }
}

// Copied from [`std::io::copy`]
pub(crate) fn generic_copy<R: ?Sized, W: ?Sized>(
    reader: &mut R,
    writer: &mut W,
    span: Span,
    interrupt: &AtomicBool,
) -> Result<u64, ShellError>
where
    R: Read,
    W: Write,
{
    let buf = &mut [0; DEFAULT_BUF_SIZE];
    let mut len = 0;
    loop {
        if interrupt.load(Ordering::Relaxed) {
            return Err(ShellError::InterruptedByUser { span: Some(span) });
        }
        let n = match reader.read(buf) {
            Ok(0) => break,
            Ok(n) => n,
            Err(e) if e.kind() == ErrorKind::Interrupted => continue,
            Err(e) => return Err(e.into_spanned(span).into()),
        };
        len += n;
        writer.write_all(&buf[..n]).err_span(span)?;
    }
    Ok(len as u64)
}
