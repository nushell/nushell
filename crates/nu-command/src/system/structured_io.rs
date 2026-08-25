use nu_protocol::{
    ByteStream, ByteStreamType, PipelineData, ShellError, Signals, Span, Spanned, StructuredIoMode,
    engine::EngineState,
    shell_error::{generic::GenericError, io::IoError},
};
use nuon::{ToNuonConfig, ToStyle, from_nuon, to_nuon};
use std::{
    ffi::OsString,
    io::{BufRead, BufReader, Cursor, Read, Write},
    path::{Path, PathBuf},
};

/// Marks NUON written by a parent nu so the child does not treat raw pipes as structured.
const STRUCTURED_IO_HEADER: &[u8] = b"\x1eNUON\n";

/// Whether this spawn should use the parent/child NUON handshake.
///
/// `cli_mode` is the last `--structured-io` on a `nu` binary argv only.
pub fn structured_io_for_child(
    resolved: &Path,
    invoked: &Path,
    stdout_captured: bool,
    input: &PipelineData,
    cli_mode: Option<StructuredIoMode>,
) -> StructuredIoMode {
    if !is_nushell_child(resolved, invoked) {
        return StructuredIoMode::default();
    }
    if let Some(mode) = cli_mode {
        return mode;
    }

    let input = matches!(
        input,
        PipelineData::Value(..) | PipelineData::ListStream(..)
    );
    StructuredIoMode {
        input,
        output: stdout_captured,
    }
}

/// Last `--structured-io` on a `nu` binary argv.
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

/// How to spawn the child so it receives `--structured-io` on argv.
pub struct StructuredIoSpawn {
    pub program: PathBuf,
    pub leading_args: Vec<OsString>,
}

/// Prepend `--structured-io=` when injecting onto the `nu` binary.
///
/// Shebang scripts are spawned as-is. Skip injection when the user already
/// passed `--structured-io` so that value is the one the child parses.
///
/// `inject_off` is for `complete`/`tee`/`save`: stdout is a pipe, so the child
/// would infer structured output, but the parent must keep the raw process
/// (stderr is captured separately and cannot be drained as NUON).
pub fn structured_io_spawn(
    executable: &Path,
    mode: StructuredIoMode,
    inject_flag: bool,
    inject_off: bool,
) -> StructuredIoSpawn {
    let program = executable.to_path_buf();
    if !inject_flag || !is_nu_binary(executable) {
        return StructuredIoSpawn {
            program,
            leading_args: Vec::new(),
        };
    }
    let flag = match mode.as_flag_str() {
        Some(flag) => flag,
        None if inject_off => "false",
        None => {
            return StructuredIoSpawn {
                program,
                leading_args: Vec::new(),
            };
        }
    };
    StructuredIoSpawn {
        program,
        leading_args: vec![OsString::from(format!("--structured-io={flag}"))],
    }
}

pub fn is_nu_binary(path: &Path) -> bool {
    path.file_name()
        .and_then(|name| name.to_str())
        .is_some_and(nu_system::is_nushell_basename)
}

pub fn encode_structured_pipeline(
    engine_state: &EngineState,
    data: PipelineData,
    mut writer: impl Write,
    span: Span,
) -> Result<(), ShellError> {
    let value = data.into_value(span)?;
    if value.is_nothing() {
        return Ok(());
    }

    let nuon = to_nuon(
        engine_state,
        &value,
        ToNuonConfig::default().style(ToStyle::Raw),
    )?;
    writer
        .write_all(STRUCTURED_IO_HEADER)
        .map_err(|err| IoError::new_internal(err, "Could not write structured pipeline data"))?;
    writer
        .write_all(nuon.as_bytes())
        .map_err(|err| IoError::new_internal(err, "Could not write structured pipeline data"))?;
    if !nuon.ends_with('\n') {
        writer.write_all(b"\n").map_err(|err| {
            IoError::new_internal(err, "Could not write structured pipeline data")
        })?;
    }
    Ok(())
}

pub fn decode_structured_pipeline(
    data: PipelineData,
    span: Span,
) -> Result<PipelineData, ShellError> {
    let bytes = match data {
        PipelineData::Empty => return Ok(PipelineData::empty()),
        PipelineData::ByteStream(stream, _) => stream.into_bytes()?,
        other => {
            let value = other.into_value(span)?;
            if value.is_nothing() {
                return Ok(PipelineData::empty());
            }
            value.coerce_into_string()?.into_bytes()
        }
    };

    if bytes.is_empty() {
        return Ok(PipelineData::empty());
    }

    // Only parse NUON when the child opted in with the frame. Bare text from
    // `nu script.nu` (python, `print`, virtualenv, ...) stays a byte stream.
    if !bytes.starts_with(STRUCTURED_IO_HEADER) {
        return Ok(PipelineData::byte_stream(
            ByteStream::read(
                Cursor::new(bytes),
                span,
                Signals::empty(),
                ByteStreamType::Unknown,
            ),
            None,
        ));
    }

    let nuon = strip_structured_io_header(&bytes);
    let text = std::str::from_utf8(nuon).map_err(|err| {
        ShellError::Generic(GenericError::new(
            "Child nu structured output was not valid UTF-8",
            err.to_string(),
            span,
        ))
    })?;

    let value = from_nuon(text, Some(span)).map_err(|err| {
        ShellError::Generic(GenericError::new(
            "Failed to parse structured output from child nu",
            err.to_string(),
            span,
        ))
    })?;

    if value.is_nothing() {
        Ok(PipelineData::empty())
    } else {
        Ok(PipelineData::value(value, None))
    }
}

pub fn emit_structured_pipeline(
    engine_state: &EngineState,
    pipeline: PipelineData,
) -> Result<(), ShellError> {
    encode_structured_pipeline(
        engine_state,
        pipeline,
        std::io::stdout().lock(),
        Span::unknown(),
    )
}

pub fn read_structured_stdin() -> Result<PipelineData, ShellError> {
    match read_startup_stdin(true)? {
        StartupStdin::Structured(data) | StartupStdin::Raw(data) => Ok(data),
        StartupStdin::Empty => Ok(PipelineData::empty()),
    }
}

/// Stdin collected at process start for structured-IO inference.
pub enum StartupStdin {
    Structured(PipelineData),
    Raw(PipelineData),
    Empty,
}

/// Read stdin once. `require_nuon` fails if the bytes are not NUON.
/// Otherwise only a framed payload (`STRUCTURED_IO_HEADER`) is treated as structured.
pub fn read_startup_stdin(require_nuon: bool) -> Result<StartupStdin, ShellError> {
    use std::io::IsTerminal;

    if std::io::stdin().is_terminal() {
        return Ok(StartupStdin::Empty);
    }

    let mut buf = Vec::new();
    std::io::stdin()
        .lock()
        .read_to_end(&mut buf)
        .map_err(|err| IoError::new_internal(err, "Could not read stdin"))?;

    if buf.is_empty() {
        return Ok(StartupStdin::Empty);
    }

    let framed = buf.starts_with(STRUCTURED_IO_HEADER);
    if require_nuon || framed {
        let text = std::str::from_utf8(strip_structured_io_header(&buf)).map_err(|err| {
            ShellError::Generic(GenericError::new(
                "Structured stdin was not valid UTF-8",
                err.to_string(),
                Span::unknown(),
            ))
        })?;
        let value = from_nuon(text, None)?;
        let data = if value.is_nothing() {
            PipelineData::empty()
        } else {
            PipelineData::value(value, None)
        };
        return Ok(StartupStdin::Structured(data));
    }

    Ok(StartupStdin::Raw(PipelineData::byte_stream(
        ByteStream::read(
            Cursor::new(buf),
            Span::unknown(),
            Signals::empty(),
            ByteStreamType::Unknown,
        ),
        None,
    )))
}

fn strip_structured_io_header(bytes: &[u8]) -> &[u8] {
    bytes.strip_prefix(STRUCTURED_IO_HEADER).unwrap_or(bytes)
}

pub fn is_nushell_child(resolved: &Path, invoked: &Path) -> bool {
    if is_nu_binary(resolved) || is_nu_binary(invoked) {
        return true;
    }
    if has_nu_extension(resolved) || has_nu_extension(invoked) {
        return true;
    }
    if !invoked_as_extensionless_script(invoked) {
        return false;
    }
    shebang_invokes_nu(resolved)
}

fn has_nu_extension(path: &Path) -> bool {
    path.extension()
        .and_then(|ext| ext.to_str())
        .is_some_and(|ext| ext.eq_ignore_ascii_case("nu"))
}

fn invoked_as_extensionless_script(invoked: &Path) -> bool {
    if invoked.extension().is_some() {
        return false;
    }
    invoked.starts_with(".") || invoked.components().nth(1).is_some()
}

fn shebang_invokes_nu(path: &Path) -> bool {
    let Ok(file) = std::fs::File::open(path) else {
        return false;
    };
    let mut first = String::new();
    if BufReader::new(file).read_line(&mut first).is_err() {
        return false;
    }
    let Some(rest) = first.strip_prefix("#!") else {
        return false;
    };
    rest.split_whitespace().any(nu_system::is_nushell_basename)
}

#[cfg(test)]
mod tests {
    use super::*;
    use nu_protocol::{IntoSpanned, Value};
    use std::io::Write;
    use tempfile::NamedTempFile;

    #[test]
    fn detects_nu_basename() {
        assert!(is_nushell_child(Path::new("/usr/bin/nu"), Path::new("nu")));
        assert!(is_nushell_child(Path::new("nu.exe"), Path::new("nu.exe")));
        assert!(!is_nushell_child(Path::new("/usr/bin/ls"), Path::new("ls")));
    }

    #[test]
    fn detects_nu_extension() {
        assert!(is_nushell_child(
            Path::new("./foo.nu"),
            Path::new("./foo.nu")
        ));
    }

    #[test]
    fn path_command_does_not_open_shebang() {
        assert!(!is_nushell_child(Path::new("/usr/bin/ls"), Path::new("ls")));
    }

    #[test]
    fn detects_env_shebang() {
        let mut file = NamedTempFile::new().expect("tempfile");
        writeln!(file, "#!/usr/bin/env -S nu --stdin").expect("write");
        writeln!(file, "def main [] {{ 1 }}").expect("write");
        assert!(is_nushell_child(file.path(), file.path()));
    }

    #[test]
    fn ignores_unrelated_shebang() {
        let mut file = NamedTempFile::new().expect("tempfile");
        writeln!(file, "#!/bin/sh").expect("write");
        assert!(!is_nushell_child(file.path(), file.path()));
    }

    #[test]
    fn extensionless_bin_nu_shebang_is_nushell() {
        let mut file = NamedTempFile::new().expect("tempfile");
        writeln!(file, "#!/bin/nu").expect("write");
        assert!(is_nushell_child(file.path(), Path::new("./test")));
    }

    #[test]
    fn extensionless_sh_shebang_is_not_nushell() {
        let mut file = NamedTempFile::new().expect("tempfile");
        writeln!(file, "#!/bin/sh").expect("write");
        assert!(!is_nushell_child(file.path(), Path::new("./test")));
    }

    fn arg(s: &str) -> Spanned<OsString> {
        OsString::from(s).into_spanned(Span::test_data())
    }

    #[test]
    fn cli_mode_reads_equals_form() {
        let args = [arg("--structured-io=false"), arg("-n")];
        assert_eq!(
            structured_io_cli_mode(&args),
            Some(StructuredIoMode::default())
        );
    }

    #[test]
    fn cli_mode_reads_out() {
        let args = [arg("--structured-io=out")];
        assert_eq!(
            structured_io_cli_mode(&args),
            Some(StructuredIoMode {
                input: false,
                output: true
            })
        );
    }

    #[test]
    fn cli_mode_last_assignment_wins() {
        let args = [arg("--structured-io=out"), arg("--structured-io=false")];
        assert_eq!(
            structured_io_cli_mode(&args),
            Some(StructuredIoMode::default())
        );
    }

    #[test]
    fn shebang_script_is_spawned_as_is() {
        let spawn =
            structured_io_spawn(Path::new("./foo.nu"), StructuredIoMode::both(), true, false);
        assert_eq!(spawn.program, PathBuf::from("./foo.nu"));
        assert!(spawn.leading_args.is_empty());
    }

    #[test]
    fn nu_binary_gets_structured_io_flag() {
        let spawn = structured_io_spawn(
            Path::new("/usr/bin/nu"),
            StructuredIoMode::both(),
            true,
            false,
        );
        assert_eq!(spawn.program, PathBuf::from("/usr/bin/nu"));
        assert_eq!(
            spawn.leading_args,
            vec![OsString::from("--structured-io=1")]
        );
    }

    #[test]
    fn injects_false_when_inference_must_be_suppressed() {
        let spawn = structured_io_spawn(
            Path::new("/usr/bin/nu"),
            StructuredIoMode::default(),
            true,
            true,
        );
        assert_eq!(
            spawn.leading_args,
            vec![OsString::from("--structured-io=false")]
        );
    }

    #[test]
    fn does_not_inject_when_user_set_flag() {
        let spawn = structured_io_spawn(
            Path::new("/usr/bin/nu"),
            StructuredIoMode::both(),
            false,
            false,
        );
        assert!(spawn.leading_args.is_empty());
    }

    #[test]
    fn script_args_do_not_set_cli_mode_for_shebang() {
        let auto = structured_io_for_child(
            Path::new("./foo.nu"),
            Path::new("./foo.nu"),
            true,
            &PipelineData::empty(),
            None,
        );
        assert!(auto.output);
        assert!(!auto.input);
    }

    #[test]
    fn decode_unframed_bytes_stays_byte_stream() {
        let data = PipelineData::byte_stream(
            ByteStream::read(
                Cursor::new(b"created virtual environment\n"),
                Span::test_data(),
                Signals::empty(),
                ByteStreamType::Unknown,
            ),
            None,
        );
        let out = decode_structured_pipeline(data, Span::test_data()).expect("decode");
        assert!(matches!(out, PipelineData::ByteStream(..)));
    }

    #[test]
    fn decode_framed_nuon_is_a_value() {
        let mut raw = STRUCTURED_IO_HEADER.to_vec();
        raw.extend_from_slice(b"[1, 2, 3]\n");
        let data = PipelineData::byte_stream(
            ByteStream::read(
                Cursor::new(raw),
                Span::test_data(),
                Signals::empty(),
                ByteStreamType::Unknown,
            ),
            None,
        );
        let out = decode_structured_pipeline(data, Span::test_data()).expect("decode");
        let value = out.into_value(Span::test_data()).expect("value");
        assert_eq!(
            value,
            Value::test_list(vec![
                Value::test_int(1),
                Value::test_int(2),
                Value::test_int(3),
            ])
        );
    }
}
