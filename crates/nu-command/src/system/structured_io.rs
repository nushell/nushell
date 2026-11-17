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
//! gets the pipe itself and reads the frame. Any other program, and any command that collects the
//! output (`print`, `$in`), gets it decoded, through the decoder this crate sets on the engine.
//!
//! A frame is [`STRUCTURED_IO_HEADER`], then the value as raw NUON, then a newline. The child
//! writes one only for the result of its commands or script, and only when nothing else has gone
//! to its stdout, startup files included. Otherwise it prints its value as text, as it does
//! without a parent `nu`, and the parent passes that text on as a byte stream. An older `nu`
//! ignores the variable, so it falls back to text the same way.

use super::run_external::shebang_words;
use nu_protocol::{
    ByteStream, ByteStreamType, OutDest, PipelineData, PipelineMetadata,
    STRUCTURED_IO_CONTENT_TYPE, STRUCTURED_IO_ENV, ShellError, Signals, Span, Spanned,
    StructuredIoMode, Value,
    engine::EngineState,
    is_structured_io_child,
    process::{ChildPipe, ChildProcess},
    shell_error::{generic::GenericError, io::IoError},
};
use nuon::{ToNuonConfig, ToStyle, from_nuon, to_nuon};
use std::{
    borrow::Cow,
    ffi::OsString,
    io::{Cursor, ErrorKind, IsTerminal, Read, Write},
    iter::Peekable,
    path::Path,
    process::Command,
};

/// Starts every frame. `0x1E` is the ASCII record separator, which text output almost never
/// starts with.
const STRUCTURED_IO_HEADER: &[u8] = b"\x1eNUON\n";

/// How a `nu` flag takes its value, so a scan of a child's command line can step over it.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum NuFlagValue {
    /// One value: `--config x.nu`, `--config=x.nu`, `-m rounded` or `-mrounded`.
    One,
    /// The values up to the next argument that starts with `-`: `--log-include a b`.
    List,
    /// One value, or a `[...]` list spread over several arguments: `--experimental-options`.
    BracketList,
}

/// Every flag of the `nu` binary that takes a value: long name, short name and how it takes the
/// value. A test in the `nu` binary checks this against its own flag table.
pub const NU_VALUE_FLAGS: &[(&str, Option<char>, NuFlagValue)] = &[
    ("commands", Some('c'), NuFlagValue::One),
    ("execute", Some('e'), NuFlagValue::One),
    ("include-path", Some('I'), NuFlagValue::One),
    ("table-mode", Some('m'), NuFlagValue::One),
    ("error-style", None, NuFlagValue::One),
    ("config", None, NuFlagValue::One),
    ("env-config", None, NuFlagValue::One),
    ("config-home", None, NuFlagValue::One),
    ("log-level", None, NuFlagValue::One),
    ("log-target", None, NuFlagValue::One),
    ("log-file", None, NuFlagValue::One),
    ("log-include", None, NuFlagValue::List),
    ("log-exclude", None, NuFlagValue::List),
    ("structured-io", None, NuFlagValue::One),
    ("experimental-options", None, NuFlagValue::BracketList),
    ("ide-goto-def", None, NuFlagValue::One),
    ("ide-hover", None, NuFlagValue::One),
    ("ide-complete", None, NuFlagValue::One),
    ("ide-check", None, NuFlagValue::One),
    ("plugin-config", None, NuFlagValue::One),
    ("plugins", None, NuFlagValue::One),
    ("mcp-transport", None, NuFlagValue::One),
    ("mcp-port", None, NuFlagValue::One),
    ("mcp-host", None, NuFlagValue::One),
    ("mcp-allowed-hosts", None, NuFlagValue::List),
    ("mcp-allowed-origins", None, NuFlagValue::List),
];

/// Which sides of the handshake to use for one spawn.
///
/// `resolved` is the executable found on `PATH` and `invoked` is the name it was called by.
/// `cli_mode` is what the command line of a `nu` binary asks for (see
/// [`structured_io_cli_mode`]); the child follows its flags over [`STRUCTURED_IO_ENV`], so the
/// parent does too.
pub fn structured_io_for_child(
    resolved: &Path,
    invoked: &Path,
    stdout: &OutDest,
    stderr: &OutDest,
    input: &PipelineData,
    cli_mode: Option<StructuredIoMode>,
) -> StructuredIoMode {
    // Only stdout on a pipe of its own can carry a frame, and only while nothing captures stderr.
    // `complete`, `save` and `tee` ask for `PipeSeparate` because they handle the raw output,
    // `o+e>|` merges stderr into the pipe, and a captured stderr (`nu --mcp` captures both) would
    // need reading alongside stdout, with no place to keep it once stdout is a value.
    // `PipeRaw` (a plain `|` into an external) still gets the value: the next command may be
    // another `nu`, which takes it as a frame, and any other program gets it as text.
    let stderr_captured = matches!(
        stderr,
        OutDest::Pipe | OutDest::PipeRaw | OutDest::PipeSeparate | OutDest::Value
    );
    let wanted = StructuredIoMode {
        input: matches!(
            input,
            PipelineData::Value(..) | PipelineData::ListStream(..)
        ) || is_structured_io_child(input),
        output: matches!(stdout, OutDest::Pipe | OutDest::PipeRaw | OutDest::Value)
            && !stderr_captured,
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

/// What the command line of a `nu` binary asks of the handshake.
///
/// Off when the child won't take part: a server mode (`--mcp`, `--lsp`, `--dap`), whose stdin and
/// stdout carry a protocol, or neither commands nor a script to run. Otherwise the last
/// `--structured-io`, or `None` when there is none.
///
/// The child reads its own flags only up to the script path or through the commands of `-c`, and
/// passes the rest on to the script, so the scan stops there too. Getting there means stepping
/// over flag values the way the child's parser does (see [`NU_VALUE_FLAGS`]).
pub fn structured_io_cli_mode(args: &[Spanned<OsString>]) -> Option<StructuredIoMode> {
    let mut args = args.iter().map(|arg| arg.item.to_string_lossy()).peekable();
    let mut mode = None;
    let mut runs = false;
    let mut server = false;
    while let Some(arg) = args.next() {
        if arg == "--" {
            // The script path follows.
            runs = args.next().is_some();
            break;
        }
        let (name, value) = if let Some(long) = arg.strip_prefix("--") {
            if matches!(long, "mcp" | "lsp" | "dap") {
                server = true;
                continue;
            }
            let (name, inline) = match long.split_once('=') {
                Some((name, value)) => (name, Some(value.to_owned())),
                None => (long, None),
            };
            let Some(&(_, _, kind)) = NU_VALUE_FLAGS.iter().find(|flag| flag.0 == name) else {
                continue;
            };
            (name, take_flag_value(kind, inline, &mut args))
        } else if let Some(shorts) = arg.strip_prefix('-')
            && !shorts.is_empty()
        {
            // In a group (`-nc`), the first flag that takes a value takes the rest of the group,
            // or the next argument when nothing is left.
            let found = shorts.char_indices().find_map(|(at, short)| {
                let flag = NU_VALUE_FLAGS.iter().find(|flag| flag.1 == Some(short))?;
                Some((&shorts[at + short.len_utf8()..], flag))
            });
            let Some((rest, &(name, _, kind))) = found else {
                continue;
            };
            let inline = (!rest.is_empty()).then(|| rest.strip_prefix('=').unwrap_or(rest).into());
            (name, take_flag_value(kind, inline, &mut args))
        } else {
            // The script path (or on Windows, `/c` for `-c`). The rest belongs to the script.
            runs = true;
            break;
        };
        match name {
            "commands" => {
                runs = true;
                break;
            }
            "structured-io" => mode = value.map(|value| StructuredIoMode::from_flag_str(&value)),
            _ => {}
        }
    }
    if server || !runs {
        Some(StructuredIoMode::default())
    } else {
        mode
    }
}

/// Takes a flag's value from `args`, the way the `nu` binary's parser does, after the value given
/// in the flag's own argument (`--flag=value`, `-mvalue`), if any. Returns the value of a
/// [`NuFlagValue::One`] flag; the others are only stepped over.
fn take_flag_value<'a>(
    kind: NuFlagValue,
    inline: Option<String>,
    args: &mut Peekable<impl Iterator<Item = Cow<'a, str>>>,
) -> Option<String> {
    match kind {
        NuFlagValue::One => inline.or_else(|| args.next().map(Cow::into_owned)),
        // A value in the flag's own argument is the only one.
        NuFlagValue::List => {
            if inline.is_none() {
                while args
                    .next_if(|next| !next.starts_with('-') || next == "-")
                    .is_some()
                {}
            }
            None
        }
        NuFlagValue::BracketList => {
            let first = inline.or_else(|| args.next().map(Cow::into_owned))?;
            if first.trim_start().starts_with('[') && !first.trim_end().ends_with(']') {
                while args
                    .next()
                    .is_some_and(|next| !next.trim_end().ends_with(']'))
                {}
            }
            None
        }
    }
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

/// Whether a spawn runs Nushell: the `nu` binary, or a script that `nu` runs. On Unix the
/// shebang line names the program that runs a script, whatever the script is called, so the
/// `.nu` extension counts only for a file without one.
pub fn is_nushell_child(resolved: &Path, invoked: &Path) -> bool {
    if is_nu_binary(resolved) || is_nu_binary(invoked) {
        return true;
    }
    if cfg!(unix)
        && let Some(words) = shebang_words(resolved)
    {
        return words
            .iter()
            .filter_map(|word| Path::new(word).file_name()?.to_str())
            .any(is_nushell_basename);
    }
    has_nu_extension(resolved) || has_nu_extension(invoked)
}

/// Writes `value` to `writer` as one frame. Returns `false`, having written nothing, when the
/// value has no NUON form (a closure, for one).
fn write_frame(
    engine_state: &EngineState,
    value: &Value,
    writer: &mut impl Write,
) -> std::io::Result<bool> {
    let Ok(nuon) = to_nuon(
        engine_state,
        value,
        ToNuonConfig::default().style(ToStyle::Raw),
    ) else {
        return Ok(false);
    };
    writer.write_all(STRUCTURED_IO_HEADER)?;
    writer.write_all(nuon.as_bytes())?;
    if !nuon.ends_with('\n') {
        writer.write_all(b"\n")?;
    }
    writer.flush()?;
    Ok(true)
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
    let written = write_frame(engine_state, &value, writer)
        .map_err(|err| IoError::new_internal(err, "Could not write structured input"))?;
    Ok((!written).then_some(value))
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

/// Decodes the output from [`tag_structured_child`] like [`read_structured_child`] does, for a
/// command that doesn't read frames. Anything else passes through.
///
/// This is the decoder set on the engine (see
/// [`EngineState::decode_structured_io`](nu_protocol::engine::EngineState::decode_structured_io)).
pub fn decode_structured_child(data: PipelineData, span: Span) -> Result<PipelineData, ShellError> {
    match data {
        PipelineData::ByteStream(stream, Some(metadata))
            if metadata.content_type.as_deref() == Some(STRUCTURED_IO_CONTENT_TYPE) =>
        {
            match stream.into_child() {
                Ok(child) => read_structured_child(child, span),
                // Tagged with `metadata set`, not by `run-external`: there is no child to read.
                Err(stream) => Ok(PipelineData::byte_stream(stream, Some(metadata))),
            }
        }
        data => Ok(data),
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
    let written = write_frame(engine_state, &value, &mut std::io::stdout().lock())
        .map_err(|err| IoError::new_internal(err, "Could not write structured output"))?;
    Ok((!written).then(|| PipelineData::value(value, metadata)))
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

    #[cfg(unix)]
    #[test]
    fn other_shebang_beats_nu_extension() {
        let mut file = tempfile::Builder::new()
            .suffix(".nu")
            .tempfile()
            .expect("tempfile");
        writeln!(file, "#!/usr/bin/env python3").expect("write");
        assert!(!is_nushell_child(file.path(), file.path()));
    }

    fn args(args: &[&str]) -> Vec<Spanned<OsString>> {
        args.iter()
            .map(|arg| OsString::from(arg).into_spanned(Span::test_data()))
            .collect()
    }

    #[test]
    fn cli_mode_last_assignment_wins() {
        let line = args(&[
            "--structured-io=out",
            "-n",
            "--structured-io",
            "false",
            "-c",
            "1",
        ]);
        assert_eq!(
            structured_io_cli_mode(&line),
            Some(StructuredIoMode::default())
        );
        assert_eq!(structured_io_cli_mode(&args(&["-n", "-c", "1"])), None);
        assert_eq!(structured_io_cli_mode(&args(&["-n", "foo.nu"])), None);
    }

    #[test]
    fn cli_mode_stops_where_the_script_takes_over() {
        let off = Some(StructuredIoMode::default());
        for line in [
            &["foo.nu", "--structured-io=false"][..],
            &["-c", "1", "--structured-io=false"],
            &["-nc", "1", "--structured-io=false"],
            &["-c1", "--structured-io=false"],
            &["--commands=1", "--structured-io=false"],
            &["--", "foo.nu", "--structured-io=false"],
            // Flag values are not the script.
            &["--config", "x.nu", "foo.nu", "--structured-io=false"],
            &["-m", "rounded", "foo.nu", "--structured-io=false"],
            &["-mrounded", "foo.nu", "--structured-io=false"],
            // A list takes everything up to the next flag, as the child's parser does.
            &[
                "--log-include",
                "a",
                "b",
                "-n",
                "foo.nu",
                "--structured-io=false",
            ],
            &[
                "--experimental-options",
                "[a,",
                "b]",
                "foo.nu",
                "--structured-io=false",
            ],
        ] {
            assert_eq!(structured_io_cli_mode(&args(line)), None, "{line:?}");
        }
        for line in [
            &["--config", "x.nu", "--structured-io=false", "foo.nu"][..],
            &["--log-include", "a", "--structured-io", "false", "foo.nu"],
            &[
                "--experimental-options=[a,b]",
                "--structured-io=false",
                "foo.nu",
            ],
            &["--structured-io=false", "-c", "1"],
        ] {
            assert_eq!(structured_io_cli_mode(&args(line)), off, "{line:?}");
        }
    }

    #[test]
    fn cli_mode_is_off_without_commands_or_script() {
        let off = Some(StructuredIoMode::default());
        for line in [
            &[][..],
            &["-n"],
            &["--config", "x.nu"],
            &["--mcp"],
            &["--lsp", "foo.nu"],
            &["--dap", "-c", "1"],
            &["--log-include", "a", "b"],
            &["--log-include", "a", "foo.nu"],
        ] {
            assert_eq!(structured_io_cli_mode(&args(line)), off, "{line:?}");
        }
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
    fn no_output_while_stderr_is_captured() {
        let empty = PipelineData::empty;
        // `nu --mcp` captures both.
        assert!(!mode_for(OutDest::Value, OutDest::Value, empty(), None).output);
        assert!(!mode_for(OutDest::PipeRaw, OutDest::Pipe, empty(), None).output);
        assert!(!mode_for(OutDest::Pipe, OutDest::PipeSeparate, empty(), None).output);
        assert!(mode_for(OutDest::Pipe, OutDest::Null, empty(), None).output);
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
        let mut frame = Vec::new();
        assert!(write_frame(&engine_state, &value, &mut frame).expect("write"));
        assert!(frame.starts_with(STRUCTURED_IO_HEADER));
        let decoded = parse_frame(&frame, Span::test_data())
            .and_then(|data| data.into_value(Span::test_data()))
            .expect("decode");
        assert_eq!(decoded, value);
    }

    #[test]
    fn value_without_nuon_writes_nothing() {
        let engine_state = EngineState::new();
        let closure = Value::test_closure(nu_protocol::engine::Closure {
            block_id: nu_protocol::BlockId::new(0),
            captures: vec![],
        });
        let mut frame = Vec::new();
        assert!(!write_frame(&engine_state, &closure, &mut frame).expect("write"));
        assert!(frame.is_empty());
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
