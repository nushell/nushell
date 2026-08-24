use nu_experimental::StructuredIoMode;
use nu_protocol::{
    PipelineData, ShellError, Span, Spanned,
    engine::EngineState,
    shell_error::{generic::GenericError, io::IoError},
};
use nuon::{ToNuonConfig, ToStyle, from_nuon, to_nuon};
use std::{
    ffi::OsString,
    io::{BufRead, BufReader, Write},
    path::{Path, PathBuf},
};

/// Whether this spawn should use the parent/child NUON handshake.
///
/// Always on for Nushell children unless argv has `--structured-io=false`.
pub fn structured_io_for_child(
    executable: &Path,
    stdout_captured: bool,
    input: &PipelineData,
    cli_override: Option<bool>,
) -> StructuredIoMode {
    if !is_nushell_child(executable) {
        return StructuredIoMode::default();
    }
    if cli_override == Some(false) {
        return StructuredIoMode::default();
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

/// Last `--structured-io=` on the child argv. `Some(false)` disables the handshake.
pub fn structured_io_cli_override(args: &[Spanned<OsString>]) -> Option<bool> {
    let mut last = None;
    let mut items = args.iter().map(|arg| arg.item.to_string_lossy());
    while let Some(arg) = items.next() {
        if arg == "--structured-io" {
            if let Some(value) = items.next() {
                last = Some(flag_enables_structured_io(&value));
            }
        } else if let Some(value) = arg.strip_prefix("--structured-io=") {
            last = Some(flag_enables_structured_io(value));
        }
    }
    last
}

fn flag_enables_structured_io(value: &str) -> bool {
    !matches!(value.trim(), "false" | "off" | "0")
}

/// How to spawn the child so it receives `--structured-io` on argv, not in the environment.
pub struct StructuredIoSpawn {
    pub program: PathBuf,
    pub leading_args: Vec<OsString>,
}

/// Prepend `--structured-io=` when the child is the `nu` binary.
///
/// Shebang scripts (`./foo.nu`) are spawned as-is. The child infers the parent
/// process is Nushell and enables structured IO itself.
pub fn structured_io_spawn(executable: &Path, mode: StructuredIoMode) -> StructuredIoSpawn {
    let Some(flag) = mode.as_flag_str() else {
        return StructuredIoSpawn {
            program: executable.to_path_buf(),
            leading_args: Vec::new(),
        };
    };
    if is_nu_binary(executable) {
        return StructuredIoSpawn {
            program: executable.to_path_buf(),
            leading_args: vec![OsString::from(format!("--structured-io={flag}"))],
        };
    }
    StructuredIoSpawn {
        program: executable.to_path_buf(),
        leading_args: Vec::new(),
    }
}

fn is_nu_binary(path: &Path) -> bool {
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

    let text = String::from_utf8(bytes).map_err(|err| {
        ShellError::Generic(GenericError::new(
            "Child nu structured output was not valid UTF-8",
            err.to_string(),
            span,
        ))
    })?;

    let value = from_nuon(&text, Some(span)).map_err(|err| {
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
    use std::io::{IsTerminal, Read};

    if std::io::stdin().is_terminal() {
        return Ok(PipelineData::empty());
    }

    let mut buf = String::new();
    std::io::stdin()
        .lock()
        .read_to_string(&mut buf)
        .map_err(|err| IoError::new_internal(err, "Could not read structured stdin"))?;

    if buf.is_empty() {
        return Ok(PipelineData::empty());
    }

    let value = from_nuon(&buf, None)?;
    if value.is_nothing() {
        Ok(PipelineData::empty())
    } else {
        Ok(PipelineData::value(value, None))
    }
}

pub fn is_nushell_child(path: &Path) -> bool {
    if path
        .file_name()
        .and_then(|name| name.to_str())
        .is_some_and(nu_system::is_nushell_basename)
    {
        return true;
    }
    if path
        .extension()
        .and_then(|ext| ext.to_str())
        .is_some_and(|ext| ext.eq_ignore_ascii_case("nu"))
    {
        return true;
    }
    shebang_invokes_nu(path)
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
    use nu_protocol::IntoSpanned;
    use std::io::Write;
    use tempfile::NamedTempFile;

    #[test]
    fn detects_nu_basename() {
        assert!(is_nushell_child(Path::new("/usr/bin/nu")));
        assert!(is_nushell_child(Path::new("nu.exe")));
        assert!(!is_nushell_child(Path::new("/usr/bin/ls")));
    }

    #[test]
    fn detects_nu_extension() {
        assert!(is_nushell_child(Path::new("./foo.nu")));
    }

    #[test]
    fn detects_env_shebang() {
        let mut file = NamedTempFile::new().expect("tempfile");
        writeln!(file, "#!/usr/bin/env -S nu --stdin").expect("write");
        writeln!(file, "def main [] {{ 1 }}").expect("write");
        assert!(is_nushell_child(file.path()));
    }

    #[test]
    fn ignores_unrelated_shebang() {
        let mut file = NamedTempFile::new().expect("tempfile");
        writeln!(file, "#!/bin/sh").expect("write");
        assert!(!is_nushell_child(file.path()));
    }

    fn arg(s: &str) -> Spanned<OsString> {
        OsString::from(s).into_spanned(Span::test_data())
    }

    #[test]
    fn cli_override_reads_equals_form() {
        let args = [arg("--structured-io=false"), arg("-n")];
        assert_eq!(structured_io_cli_override(&args), Some(false));
    }

    #[test]
    fn cli_override_reads_out() {
        let args = [arg("--structured-io=out")];
        assert_eq!(structured_io_cli_override(&args), Some(true));
    }

    #[test]
    fn cli_override_last_assignment_wins() {
        let args = [arg("--structured-io=out"), arg("--structured-io=false")];
        assert_eq!(structured_io_cli_override(&args), Some(false));
    }

    #[test]
    fn shebang_script_is_spawned_as_is() {
        let spawn = structured_io_spawn(Path::new("./foo.nu"), StructuredIoMode::both());
        assert_eq!(spawn.program, PathBuf::from("./foo.nu"));
        assert!(spawn.leading_args.is_empty());
    }

    #[test]
    fn nu_binary_gets_structured_io_flag() {
        let spawn = structured_io_spawn(Path::new("/usr/bin/nu"), StructuredIoMode::both());
        assert_eq!(spawn.program, PathBuf::from("/usr/bin/nu"));
        assert_eq!(
            spawn.leading_args,
            vec![OsString::from("--structured-io=1")]
        );
    }
}
