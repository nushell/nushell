//! Lets a `nu` that pipes into or out of another `nu` pass values instead of text.
//!
//! When `run-external` spawns a Nushell child (the `nu` binary, a `.nu` script, or a script whose
//! shebang runs `nu`), it sets [`STRUCTURED_IO_ENV`] on that one spawn:
//!
//! - `in`: the parent writes its input to the child's stdin as one frame.
//! - `out`: the parent reads the child's stdout and decodes a frame at its start.
//!
//! When the next command is an external, the parent doesn't wait to decode: that would hold up
//! starting it. The output goes on tagged instead (see [`tag_structured_child`]). Another `nu`
//! gets the pipe itself and reads the frame, and any other program gets it decoded into text.
//!
//! A frame is [`STRUCTURED_IO_HEADER`], then the value as raw NUON, then a newline. The child
//! writes one only when nothing else has gone to its stdout. Otherwise it prints its value as text,
//! as it does without a parent `nu`, and the parent passes that text on as a byte stream. An older
//! `nu` ignores the variable, so it falls back to text the same way.

use nu_protocol::{
    ByteStream, ByteStreamSource, ByteStreamType, OutDest, PipelineData, PipelineMetadata,
    STRUCTURED_IO_CONTENT_TYPE, STRUCTURED_IO_ENV, ShellError, Signals, Span, Spanned,
    StructuredIoMode, Value,
    engine::EngineState,
    process::{ChildPipe, ChildProcess},
    shell_error::{generic::GenericError, io::IoError},
};
use nuon::{ToNuonConfig, ToStyle, from_nuon, to_nuon};
use std::{
    ffi::OsString,
    io::{Cursor, ErrorKind, IsTerminal, Read, Write},
    path::Path,
    process::Command,
};

/// Starts every frame. `0x1E` is the ASCII record separator, which text output almost never
/// starts with.
const STRUCTURED_IO_HEADER: &[u8] = b"\x1eNUON\n";

/// Which sides of the handshake to use for one spawn.
///
/// `resolved` is the executable found on `PATH` and `invoked` is the name it was called by.
/// `cli_mode` is the last `--structured-io` on the command line of a `nu` binary; the child follows
/// that flag over [`STRUCTURED_IO_ENV`], so the parent does too.
pub fn structured_io_for_child(
    resolved: &Path,
    invoked: &Path,
    stdout: &OutDest,
    stderr: &OutDest,
    input: &PipelineData,
    cli_mode: Option<StructuredIoMode>,
) -> StructuredIoMode {
    // Only stdout on a pipe of its own can carry a frame. `complete`, `save` and `tee` ask for
    // `PipeSeparate` because they handle the raw output, and `o+e>|` merges stderr into the pipe.
    // `PipeRaw` (a plain `|` into an external) still gets the value: the next command may be
    // another `nu`, which takes it as a frame, and any other program gets it as text.
    let stderr_merged = matches!(stdout, OutDest::Pipe) && matches!(stderr, OutDest::Pipe);
    let wanted = StructuredIoMode {
        input: matches!(
            input,
            PipelineData::Value(..) | PipelineData::ListStream(..)
        ) || is_tagged_structured_child(input),
        output: matches!(stdout, OutDest::Pipe | OutDest::PipeRaw | OutDest::Value)
            && !stderr_merged,
    };
    let mode = match cli_mode {
        Some(mode) => StructuredIoMode {
            input: wanted.input && mode.input,
            output: wanted.output && mode.output,
        },
        None => wanted,
    };
    // Checked last: on Unix it reads the start of the executable to look for a shebang.
    if mode.any() && is_nushell_child(resolved, invoked) {
        mode
    } else {
        StructuredIoMode::default()
    }
}

/// Last `--structured-io` on a `nu` binary's command line.
pub fn structured_io_cli_mode(args: &[Spanned<OsString>]) -> Option<StructuredIoMode> {
    let mut last = None;
    let mut items = args.iter().map(|arg| arg.item.to_string_lossy());
    while let Some(arg) = items.next() {
        if arg == "--structured-io" {
            if let Some(value) = items.next() {
                last = Some(StructuredIoMode::from_flag_str(&value));
            }
        } else if let Some(value) = arg.strip_prefix("--structured-io=") {
            last = Some(StructuredIoMode::from_flag_str(value));
        }
    }
    last
}

/// Sets [`STRUCTURED_IO_ENV`] on `command` for `mode`, or removes it when `mode` is off, so a
/// variable the user set never reaches a child that this process won't decode.
pub fn set_structured_io_env(command: &mut Command, mode: StructuredIoMode) {
    match mode.as_flag_str() {
        Some(value) => command.env(STRUCTURED_IO_ENV, value),
        None => command.env_remove(STRUCTURED_IO_ENV),
    };
}

/// Whether `path` names the `nu` binary.
pub fn is_nu_binary(path: &Path) -> bool {
    path.file_name()
        .and_then(|name| name.to_str())
        .is_some_and(is_nushell_basename)
}

/// Whether a spawn runs Nushell: the `nu` binary, a `.nu` script, or (on Unix) a script whose
/// shebang line runs `nu`.
pub fn is_nushell_child(resolved: &Path, invoked: &Path) -> bool {
    is_nu_binary(resolved)
        || is_nu_binary(invoked)
        || has_nu_extension(resolved)
        || has_nu_extension(invoked)
        || (cfg!(unix) && shebang_invokes_nu(resolved))
}

/// The frame for `value`, or `None` when the value has no NUON form (a closure, for one).
fn structured_io_frame(engine_state: &EngineState, value: &Value) -> Option<Vec<u8>> {
    let nuon = to_nuon(
        engine_state,
        value,
        ToNuonConfig::default().style(ToStyle::Raw),
    )
    .ok()?;
    let mut frame = Vec::with_capacity(STRUCTURED_IO_HEADER.len() + nuon.len() + 1);
    frame.extend_from_slice(STRUCTURED_IO_HEADER);
    frame.extend_from_slice(nuon.as_bytes());
    if !nuon.ends_with('\n') {
        frame.push(b'\n');
    }
    Some(frame)
}

/// Writes `value` to a child's stdin as one frame, or nothing for `nothing`.
///
/// Returns the value back when it has no NUON form, so the caller can send it as text instead.
/// The child reads input that doesn't start with a frame as a byte stream.
pub fn write_structured_input(
    engine_state: &EngineState,
    value: Value,
    writer: &mut impl Write,
) -> Result<Option<Value>, ShellError> {
    if value.is_nothing() {
        return Ok(None);
    }
    let Some(frame) = structured_io_frame(engine_state, &value) else {
        return Ok(Some(value));
    };
    writer
        .write_all(&frame)
        .map_err(|err| IoError::new_internal(err, "Could not write structured input"))?;
    Ok(None)
}

/// Reads the start of a child's stdout to see whether it sent a frame.
///
/// A frame becomes the value it holds. Anything else stays the child's output, as a byte stream
/// that starts with the bytes read here, so text keeps streaming. Only the first byte is needed to
/// tell text from a frame.
pub fn read_structured_child(
    mut child: ChildProcess,
    span: Span,
) -> Result<PipelineData, ShellError> {
    let Some(mut stdout) = child.stdout.take() else {
        return Ok(PipelineData::byte_stream(
            ByteStream::child(child, span),
            None,
        ));
    };
    let prefix = read_header_prefix(&mut stdout);
    let framed = prefix == STRUCTURED_IO_HEADER;
    child.stdout = Some(ChildPipe::Tee(Box::new(Cursor::new(prefix).chain(stdout))));
    let stream = ByteStream::child(child, span);
    if !framed {
        return Ok(PipelineData::byte_stream(stream, None));
    }
    // `into_bytes` waits for the child, so a failing child is still reported as one.
    parse_frame(&stream.into_bytes()?, span)
}

/// A Nushell child's output for an external, which may start with a frame. Unlike
/// [`read_structured_child`], this doesn't wait for the child, so both commands run at once.
pub fn tag_structured_child(child: ChildProcess, span: Span) -> PipelineData {
    let metadata =
        PipelineMetadata::default().with_content_type(Some(STRUCTURED_IO_CONTENT_TYPE.into()));
    PipelineData::byte_stream(ByteStream::child(child, span), Some(metadata))
}

/// Whether `data` came from [`tag_structured_child`].
pub fn is_tagged_structured_child(data: &PipelineData) -> bool {
    matches!(data, PipelineData::ByteStream(stream, Some(metadata))
        if metadata.content_type.as_deref() == Some(STRUCTURED_IO_CONTENT_TYPE)
            && matches!(stream.source(), ByteStreamSource::Child(..)))
}

/// Decodes the output from [`tag_structured_child`] like [`read_structured_child`] does, for a
/// command that doesn't read frames. Anything else passes through.
pub fn decode_structured_child(data: PipelineData, span: Span) -> Result<PipelineData, ShellError> {
    if !is_tagged_structured_child(&data) {
        return Ok(data);
    }
    let PipelineData::ByteStream(stream, _) = data else {
        return Ok(data);
    };
    match stream.into_child() {
        Ok(child) => read_structured_child(child, span),
        Err(stream) => Ok(PipelineData::byte_stream(stream, None)),
    }
}

/// Writes the final value of this process as a frame for the parent `nu`, when it can.
///
/// Returns the pipeline back when it has to be printed as text instead: something else already
/// went to stdout, it's a byte stream (an external's output or `open --raw`), or the value has no
/// NUON form. A `nothing` value writes nothing.
pub fn emit_structured_pipeline(
    engine_state: &EngineState,
    mut pipeline: PipelineData,
) -> Result<Option<PipelineData>, ShellError> {
    if engine_state.stdout_used() || matches!(pipeline, PipelineData::ByteStream(..)) {
        return Ok(Some(pipeline));
    }
    let metadata = pipeline.take_metadata();
    let value = pipeline.into_value(Span::unknown())?;
    if value.is_nothing() {
        return Ok(None);
    }
    let Some(frame) = structured_io_frame(engine_state, &value) else {
        return Ok(Some(PipelineData::value(value, metadata)));
    };
    let mut stdout = std::io::stdout().lock();
    stdout
        .write_all(&frame)
        .and_then(|()| stdout.flush())
        .map_err(|err| IoError::new_internal(err, "Could not write structured output"))?;
    Ok(None)
}

/// Reads this process's stdin as input from a parent `nu`.
///
/// A frame becomes the value it holds. Other input stays a byte stream, so a parent that had to
/// send text still gets through, except with `require_nuon` (from an explicit
/// `--structured-io`), which parses it as NUON with or without the header.
pub fn read_structured_stdin(require_nuon: bool, span: Span) -> Result<PipelineData, ShellError> {
    let mut stdin = std::io::stdin();
    if stdin.is_terminal() {
        return Ok(PipelineData::empty());
    }
    let mut bytes = read_header_prefix(&mut stdin);
    if bytes.is_empty() {
        return Ok(PipelineData::empty());
    }
    if bytes != STRUCTURED_IO_HEADER && !require_nuon {
        return Ok(PipelineData::byte_stream(
            ByteStream::read(
                Cursor::new(bytes).chain(stdin),
                span,
                Signals::empty(),
                ByteStreamType::Unknown,
            ),
            None,
        ));
    }
    stdin
        .read_to_end(&mut bytes)
        .map_err(|err| IoError::new_internal(err, "Could not read stdin"))?;
    parse_frame(&bytes, span)
}

/// Parses a frame, with or without its header. `span` is unknown when reading stdin at startup.
fn parse_frame(bytes: &[u8], span: Span) -> Result<PipelineData, ShellError> {
    let nuon = bytes.strip_prefix(STRUCTURED_IO_HEADER).unwrap_or(bytes);
    let parse_error = |msg: String| {
        const ERROR: &str = "Could not read a value from another nu";
        let error = if span == Span::unknown() {
            GenericError::new_internal(ERROR, msg)
        } else {
            GenericError::new(ERROR, msg, span)
        };
        error.with_help("run it with `--structured-io=false` to pass text instead")
    };
    let text = std::str::from_utf8(nuon)
        .map_err(|err| ShellError::Generic(parse_error(err.to_string())))?;
    let value = from_nuon(text, Some(span)).map_err(|err| {
        ShellError::Generic(parse_error("it is not valid NUON".into()).with_inner([err]))
    })?;
    Ok(if value.is_nothing() {
        PipelineData::empty()
    } else {
        PipelineData::value(value, None)
    })
}

/// Reads from `reader` until it has a whole header or the bytes stop matching one, so text costs a
/// single read. A read error is left for whoever reads the rest of the stream.
fn read_header_prefix(reader: &mut impl Read) -> Vec<u8> {
    let mut prefix = Vec::with_capacity(STRUCTURED_IO_HEADER.len());
    let mut buf = [0; STRUCTURED_IO_HEADER.len()];
    while prefix.len() < STRUCTURED_IO_HEADER.len() && STRUCTURED_IO_HEADER.starts_with(&prefix) {
        let want = STRUCTURED_IO_HEADER.len() - prefix.len();
        match reader.read(&mut buf[..want]) {
            Ok(0) => break,
            Ok(n) => prefix.extend_from_slice(&buf[..n]),
            Err(err) if err.kind() == ErrorKind::Interrupted => {}
            Err(_) => break,
        }
    }
    prefix
}

fn is_nushell_basename(name: &str) -> bool {
    name.eq_ignore_ascii_case("nu") || name.eq_ignore_ascii_case("nu.exe")
}

fn has_nu_extension(path: &Path) -> bool {
    path.extension()
        .and_then(|ext| ext.to_str())
        .is_some_and(|ext| ext.eq_ignore_ascii_case("nu"))
}

/// Whether the file's first line is a shebang that runs `nu` (`#!/usr/bin/env nu`,
/// `#!/usr/bin/env -S nu --stdin`, `#!/opt/bin/nu`). Reads at most one short line, so a binary
/// costs one small read.
fn shebang_invokes_nu(path: &Path) -> bool {
    let Ok(file) = std::fs::File::open(path) else {
        return false;
    };
    let mut first = Vec::new();
    if file.take(256).read_to_end(&mut first).is_err() {
        return false;
    }
    let Some(rest) = first.strip_prefix(b"#!") else {
        return false;
    };
    let line = rest.split(|&b| b == b'\n').next().unwrap_or_default();
    String::from_utf8_lossy(line)
        .split_whitespace()
        .filter_map(|word| Path::new(word).file_name()?.to_str())
        .any(is_nushell_basename)
}

#[cfg(test)]
mod tests {
    use super::*;
    use nu_protocol::IntoSpanned;
    use std::io::Write;
    use tempfile::NamedTempFile;

    #[test]
    fn detects_nu_basename() {
        assert!(is_nushell_child(Path::new("/usr/bin/nu"), Path::new("nu")));
        assert!(is_nushell_child(Path::new("nu.exe"), Path::new("NU.EXE")));
        assert!(!is_nushell_child(Path::new("/usr/bin/ls"), Path::new("ls")));
        assert!(!is_nushell_child(
            Path::new("/usr/bin/nunit"),
            Path::new("nunit")
        ));
    }

    #[test]
    fn detects_nu_extension() {
        assert!(is_nushell_child(
            Path::new("./foo.nu"),
            Path::new("./foo.nu")
        ));
    }

    #[cfg(unix)]
    #[test]
    fn detects_nu_shebangs() {
        for shebang in [
            "#!/usr/bin/env nu",
            "#!/usr/bin/env -S nu --stdin",
            "#! /opt/homebrew/bin/nu",
            "#!/bin/nu",
        ] {
            let mut file = NamedTempFile::new().expect("tempfile");
            writeln!(file, "{shebang}").expect("write");
            writeln!(file, "def main [] {{ 1 }}").expect("write");
            // Found on PATH, so invoked without a path component.
            assert!(
                is_nushell_child(file.path(), Path::new("tool")),
                "{shebang}"
            );
        }
    }

    #[test]
    fn ignores_other_shebangs() {
        for first_line in ["#!/bin/sh", "#!/usr/bin/env nunit", "nu", ""] {
            let mut file = NamedTempFile::new().expect("tempfile");
            writeln!(file, "{first_line}").expect("write");
            writeln!(file, "nu").expect("write");
            assert!(
                !is_nushell_child(file.path(), Path::new("tool")),
                "{first_line}"
            );
        }
    }

    fn arg(s: &str) -> Spanned<OsString> {
        OsString::from(s).into_spanned(Span::test_data())
    }

    #[test]
    fn cli_mode_last_assignment_wins() {
        let args = [
            arg("--structured-io=out"),
            arg("-n"),
            arg("--structured-io"),
            arg("false"),
        ];
        assert_eq!(
            structured_io_cli_mode(&args),
            Some(StructuredIoMode::default())
        );
        assert_eq!(structured_io_cli_mode(&[arg("-n")]), None);
    }

    fn mode_for(
        stdout: OutDest,
        stderr: OutDest,
        input: PipelineData,
        cli_mode: Option<StructuredIoMode>,
    ) -> StructuredIoMode {
        structured_io_for_child(
            Path::new("/usr/bin/nu"),
            Path::new("nu"),
            &stdout,
            &stderr,
            &input,
            cli_mode,
        )
    }

    #[test]
    fn output_only_when_stdout_has_its_own_pipe() {
        let empty = PipelineData::empty;
        assert!(mode_for(OutDest::Pipe, OutDest::Inherit, empty(), None).output);
        assert!(mode_for(OutDest::Value, OutDest::Inherit, empty(), None).output);
        // `nu | ^cat` or `nu | nu`
        assert!(mode_for(OutDest::PipeRaw, OutDest::Inherit, empty(), None).output);
        // `o+e>|`
        assert!(!mode_for(OutDest::Pipe, OutDest::Pipe, empty(), None).output);
        // `complete`
        assert!(!mode_for(OutDest::PipeSeparate, OutDest::PipeSeparate, empty(), None).output);
        assert!(!mode_for(OutDest::Inherit, OutDest::Inherit, empty(), None).output);
    }

    #[test]
    fn input_only_for_values() {
        let value = || PipelineData::value(Value::test_int(1), None);
        assert!(mode_for(OutDest::Inherit, OutDest::Inherit, value(), None).input);
        assert!(
            !mode_for(
                OutDest::Inherit,
                OutDest::Inherit,
                PipelineData::empty(),
                None
            )
            .input
        );
        let off = Some(StructuredIoMode::default());
        assert!(!mode_for(OutDest::Pipe, OutDest::Inherit, value(), off).any());
    }

    #[test]
    fn other_programs_get_no_handshake() {
        let mode = structured_io_for_child(
            Path::new("/usr/bin/cat"),
            Path::new("cat"),
            &OutDest::Pipe,
            &OutDest::Inherit,
            &PipelineData::value(Value::test_int(1), None),
            None,
        );
        assert_eq!(mode, StructuredIoMode::default());
    }

    #[test]
    fn frame_roundtrips() {
        let engine_state = EngineState::new();
        let value = Value::test_list(vec![Value::test_int(1), Value::test_string("a\nb")]);
        let frame = structured_io_frame(&engine_state, &value).expect("frame");
        assert!(frame.starts_with(STRUCTURED_IO_HEADER));
        let decoded = parse_frame(&frame, Span::test_data())
            .and_then(|data| data.into_value(Span::test_data()))
            .expect("decode");
        assert_eq!(decoded, value);
    }

    #[test]
    fn header_prefix_stops_at_the_first_difference() {
        // One read, which can return up to a header's length of text.
        let mut text = Cursor::new(b"hello, world".to_vec());
        assert_eq!(read_header_prefix(&mut text), b"hello,");
        let mut framed = Cursor::new(b"\x1eNUON\n[1]".to_vec());
        assert_eq!(read_header_prefix(&mut framed), STRUCTURED_IO_HEADER);
        let mut short = Cursor::new(b"\x1eNU".to_vec());
        assert_eq!(read_header_prefix(&mut short), b"\x1eNU");
    }
}
