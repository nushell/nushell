use nu_experimental::{STRUCTURED_IO, STRUCTURED_IO_ENV, StructuredIoMode};
use nu_protocol::{
    PipelineData, ShellError, Span, Spanned,
    engine::EngineState,
    shell_error::{generic::GenericError, io::IoError},
};
use nuon::{ToNuonConfig, ToStyle, from_nuon, to_nuon};
use std::{
    ffi::OsString,
    io::{BufRead, BufReader, Write},
    path::Path,
};

/// Whether this spawn should use the parent/child NUON handshake.
///
/// `cli_override` comes from the child's `--experimental-options` arguments:
/// `Some(false)` wins even if the parent has structured-io on, `Some(true)`
/// requests the handshake even if the parent does not.
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
    if !STRUCTURED_IO.get() && cli_override != Some(true) {
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

/// Last explicit `structured-io` / `all` assignment in `--experimental-options` args.
pub fn structured_io_cli_override(args: &[Spanned<OsString>]) -> Option<bool> {
    let mut last = None;
    for assignment in experimental_option_assignments(args) {
        for item in assignment
            .trim()
            .trim_matches(['[', ']'])
            .split(',')
            .map(str::trim)
            .filter(|item| !item.is_empty())
        {
            let (key, value) = item.split_once('=').unwrap_or((item, "true"));
            match key.trim() {
                "all" => last = parse_opt_bool(value),
                "structured-io" => last = parse_opt_bool(value),
                _ => {}
            }
        }
    }
    last
}

fn parse_opt_bool(value: &str) -> Option<bool> {
    match value.trim() {
        "true" | "" => Some(true),
        "false" => Some(false),
        _ => None,
    }
}

fn experimental_option_assignments(args: &[Spanned<OsString>]) -> Vec<String> {
    let mut out = Vec::new();
    let mut items = args.iter().map(|arg| arg.item.to_string_lossy());
    while let Some(arg) = items.next() {
        if arg == "--experimental-options" {
            if let Some(value) = items.next() {
                out.push(value.into_owned());
            }
        } else if let Some(value) = arg.strip_prefix("--experimental-options=") {
            out.push(value.to_string());
        }
    }
    out
}

pub fn set_child_structured_io_env(command: &mut std::process::Command, mode: StructuredIoMode) {
    if let Some(value) = mode.as_env_str() {
        command.env(STRUCTURED_IO_ENV, value);
    }
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
    let name = path
        .file_name()
        .and_then(|name| name.to_str())
        .unwrap_or("");
    let name = name.to_ascii_lowercase();
    if name == "nu" || name == "nu.exe" {
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
    rest.split_whitespace().any(|word| {
        Path::new(word)
            .file_name()
            .and_then(|name| name.to_str())
            .is_some_and(|name| name == "nu" || name == "nu.exe")
    })
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
        let args = [
            arg("--experimental-options"),
            arg("structured-io=false"),
            arg("-n"),
        ];
        assert_eq!(structured_io_cli_override(&args), Some(false));
    }

    #[test]
    fn cli_override_reads_all() {
        let args = [arg("--experimental-options=all")];
        assert_eq!(structured_io_cli_override(&args), Some(true));
    }

    #[test]
    fn cli_override_last_assignment_wins() {
        let args = [
            arg("--experimental-options"),
            arg("[all=true, structured-io=false]"),
        ];
        assert_eq!(structured_io_cli_override(&args), Some(false));
    }
}
