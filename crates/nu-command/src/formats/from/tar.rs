use chrono::{DateTime, Utc};
use nu_protocol::{
    ast::Call,
    engine::{Command, EngineState, Stack},
    Category, IntoPipelineData, PipelineData, RawStream, Record, ShellError, Signature, Span, Type,
    Value,
};
use std::io::Read;
use tar::{Archive, Header};

#[derive(Clone)]
pub struct FromTar;

impl Command for FromTar {
    fn name(&self) -> &str {
        "from tar"
    }

    fn signature(&self) -> Signature {
        Signature::build(self.name())
            .input_output_types(vec![(Type::Binary, Type::Table(vec![]))])
            .switch("list", "List archive contents", Some('t'))
            .switch(
                "long",
                "Get all available columns for each entry (default without --list)",
                Some('l'),
            )
            .category(Category::Formats)
    }

    fn usage(&self) -> &str {
        "Extract or list files from a tape archive"
    }

    fn run(
        &self,
        _engine_state: &EngineState,
        _stack: &mut Stack,
        call: &Call,
        input: PipelineData,
    ) -> Result<PipelineData, ShellError> {
        let list = call.has_flag("list");
        let long = !list || call.has_flag("long");

        let (mut archive, input_span) = open_archive(input, call)?;

        let entries = archive.entries().map_err(|e| io_error(e, input_span))?;

        let mut result = vec![];

        for entry in entries {
            let entry = entry.map_err(|e| io_error(e, input_span))?;

            let record = entry_to_record(entry, list, long, input_span, call.head)?;

            result.push(record);
        }

        Ok(Value::list(result, call.head).into_pipeline_data())
    }
}

fn io_error(error: std::io::Error, span: Span) -> ShellError {
    ShellError::IOErrorSpanned(error.to_string(), span)
}

fn entry_to_record(
    mut entry: tar::Entry<'_, RawStream>,
    list: bool,
    long: bool,
    input_span: Span,
    tar_span: Span,
) -> Result<Value, ShellError> {
    let mut record = Record::new();

    let header = entry.header();

    let path = entry.path().map_err(|e| io_error(e, input_span))?;
    let name = path.to_string_lossy().to_string();
    record.push("name", Value::string(&name, tar_span));

    let entry_type = header_to_entry_type(header);
    record.push("type", Value::string(entry_type, tar_span));

    if long {
        let mode = header.mode().map_err(|e| io_error(e, input_span))?;
        let mode = umask::Mode::from(mode).to_string();
        record.push("mode", Value::string(mode, tar_span));

        let user = match entry.header().username() {
            Ok(Some(user)) => Value::string(user, tar_span),
            _ => {
                let uid = header.uid().map_err(|e| io_error(e, input_span))?;
                Value::int(uid as i64, tar_span)
            }
        };
        record.push("user", user);

        let group = match entry.header().groupname() {
            Ok(Some(group)) => Value::string(group, tar_span),
            _ => {
                let gid = header.gid().map_err(|e| io_error(e, input_span))?;
                Value::int(gid as i64, tar_span)
            }
        };
        record.push("group", group);

        if let Some(major) = entry
            .header()
            .device_major()
            .map_err(|e| io_error(e, input_span))?
        {
            record.push("major", Value::int(major.into(), tar_span));
        }

        if let Some(minor) = entry
            .header()
            .device_minor()
            .map_err(|e| io_error(e, input_span))?
        {
            record.push("minor", Value::int(minor.into(), tar_span));
        }
    }

    let size = header_to_size(header, input_span)?;
    record.push("size", Value::filesize(size, tar_span));

    if long {
        if let Some(gnu_header) = header.as_gnu() {
            let ctime = gnu_header.ctime().map_err(|e| io_error(e, input_span))?;
            let created = timestamp_to_date(ctime, &name, input_span)?;
            record.push("created", Value::date(created.into(), tar_span));

            let atime = gnu_header.atime().map_err(|e| io_error(e, input_span))?;
            let accessed = timestamp_to_date(atime, &name, input_span)?;
            record.push("accessed", Value::date(accessed.into(), tar_span));
        }
    }

    let mtime = header.mtime().map_err(|e| io_error(e, input_span))?;
    let modified = timestamp_to_date(mtime, &name, input_span)?;
    record.push("modified", Value::date(modified.into(), tar_span));

    if !list {
        let mut body = vec![];

        entry
            .read_to_end(&mut body)
            .map_err(|e| io_error(e, input_span))?;

        record.push("data", Value::binary(body, tar_span));
    }

    Ok(Value::record(record, tar_span))
}

fn header_to_entry_type(header: &Header) -> String {
    use tar::EntryType::*;

    match header.entry_type() {
        Regular => "file",
        Link => "link",
        Symlink => "symlink",
        Char => "char device",
        Block => "block device",
        Directory => "dir",
        Fifo => "pipe",
        Continuous => "contiguous",
        GNULongName => "GNU long name",
        GNULongLink => "GNU long link",
        GNUSparse => "GNU sparse file",
        XGlobalHeader => "pax global extension",
        XHeader => "pax local extension",
        _ => "unknown",
    }
    .to_string()
}

fn header_to_size(header: &Header, input_span: Span) -> Result<i64, ShellError> {
    let size = header.entry_size().map_err(|e| io_error(e, input_span))?;

    Ok(size as i64)
}

fn open_archive(
    input: PipelineData,
    call: &Call,
) -> Result<(Archive<RawStream>, Span), ShellError> {
    let (value, span) = to_binary_stream(input, call.head)?;

    let archive = Archive::new(value);

    Ok((archive, span))
}

fn timestamp_to_date(
    timestamp: u64,
    name: &str,
    input_span: Span,
) -> Result<DateTime<Utc>, ShellError> {
    DateTime::from_timestamp(timestamp as i64, 0).ok_or_else(|| ShellError::CantConvert {
        to_type: "datetime".into(),
        from_type: "unix timestamp".into(),
        span: input_span,
        help: Some(format!("tar file entry {name} is too far in the future")),
    })
}

fn to_binary_stream(input: PipelineData, span: Span) -> Result<(RawStream, Span), ShellError> {
    match input {
        PipelineData::Empty => Err(ShellError::PipelineEmpty { dst_span: span }),
        PipelineData::ExternalStream {
            stdout: None, span, ..
        } => Err(ShellError::TypeMismatch {
            err_message: "binary (try open -r)".into(),
            span,
        }),
        PipelineData::ExternalStream {
            stdout: Some(stdout),
            span,
            ..
        } => Ok((stdout, span)),
        _ => {
            let src_span = input.span().unwrap_or(Span::unknown());
            Err(ShellError::PipelineMismatch {
                exp_input_type: "binary stream (try open -r)".into(),
                dst_span: span,
                src_span,
            })
        }
    }
}
