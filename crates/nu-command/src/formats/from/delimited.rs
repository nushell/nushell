use csv::{ReaderBuilder, Trim};
use nu_protocol::{
    ByteStream, ListStream, PipelineData, ShellError, Signals, Span, Value,
    shell_error::bridge::ShellErrorBridge,
};
use std::collections::{HashMap, HashSet};
use std::sync::Arc;

fn from_csv_error(err: csv::Error, span: Span) -> ShellError {
    let msg = err.to_string();
    // An error from the stream being read has nothing to do with the delimited data.
    if let csv::ErrorKind::Io(err) = err.into_kind()
        && let Ok(ShellErrorBridge(err)) = ShellErrorBridge::try_from(err)
    {
        return err;
    }
    ShellError::DelimiterError { msg, span }
}

fn from_delimited_stream(
    DelimitedReaderConfig {
        separator,
        comment,
        quote,
        escape,
        noheaders,
        flexible,
        no_infer,
        trim,
    }: DelimitedReaderConfig,
    input: ByteStream,
    span: Span,
) -> Result<ListStream, ShellError> {
    let input_reader = if let Some(stream) = input.reader() {
        stream
    } else {
        return Ok(ListStream::new(std::iter::empty(), span, Signals::empty()));
    };

    let mut reader = ReaderBuilder::new()
        .has_headers(!noheaders)
        .flexible(flexible)
        .delimiter(separator as u8)
        .comment(comment.map(|c| c as u8))
        .quote(quote as u8)
        .escape(escape.map(|c| c as u8))
        .trim(trim)
        .from_reader(input_reader);

    let headers = if noheaders {
        vec![]
    } else {
        let headers: Vec<String> = reader
            .headers()
            .map_err(|err| from_csv_error(err, span))?
            .iter()
            .enumerate()
            // Name blank header cells by position, the same way `headers` does,
            // so files with several unlabeled columns still load.
            .map(|(idx, name)| match name.is_empty() {
                true => format!("column{idx}"),
                false => name.to_string(),
            })
            .collect();
        dedupe_headers(headers)
    };

    let n = headers.len();
    // With `--flexible`, a row can have more fields than the header; the
    // generated `columnN` filler names skip anything already used by the header
    // so they can't create another duplicate.
    // Shared, so cloning `columns` for every row doesn't copy the set.
    let header_names: Arc<HashSet<String>> = Arc::new(headers.iter().cloned().collect());
    let columns = headers.into_iter().chain(
        (n..)
            .map(|i| format!("column{i}"))
            .filter(move |name| !header_names.contains(name)),
    );
    let iter = reader.into_records().map(move |row| {
        let row = match row {
            Ok(row) => row,
            Err(err) => return Value::error(from_csv_error(err, span), span),
        };
        let columns = columns.clone();
        let values = row.into_iter().map(|s| {
            if no_infer {
                Value::string(s, span)
            } else if let Ok(i) = s.parse() {
                Value::int(i, span)
            } else if let Ok(f) = s.parse() {
                Value::float(f, span)
            } else {
                Value::string(s, span)
            }
        });

        Value::record(columns.zip(values).collect(), span)
    });

    Ok(ListStream::new(iter, span, Signals::empty()))
}

/// Give each repeated header name a `_N` suffix (`name`, `name_1`, `name_2`, ...).
///
/// Records with duplicate keys lose the earlier values on every later
/// name-based lookup (`get`, `to csv`, ...). A suffixed name never matches a
/// name the header row already uses, so the names it spells out stay as they
/// are. Used by `from csv`, `from tsv` and `headers`.
pub(crate) fn dedupe_headers(headers: Vec<String>) -> Vec<String> {
    let mut taken: HashSet<String> = headers.iter().cloned().collect();
    // The next suffix to try for each name, so a repeat picks up where the
    // previous one stopped instead of probing `_1`, `_2`, ... again.
    let mut next_suffix: HashMap<String, usize> = HashMap::with_capacity(headers.len());
    headers
        .into_iter()
        .map(|name| {
            let Some(n) = next_suffix.get_mut(&name) else {
                next_suffix.insert(name.clone(), 1);
                return name;
            };
            loop {
                let candidate = format!("{name}_{n}");
                *n += 1;
                if taken.insert(candidate.clone()) {
                    return candidate;
                }
            }
        })
        .collect()
}

pub(super) struct DelimitedReaderConfig {
    pub separator: char,
    pub comment: Option<char>,
    pub quote: char,
    pub escape: Option<char>,
    pub noheaders: bool,
    pub flexible: bool,
    pub no_infer: bool,
    pub trim: Trim,
}

pub(super) fn from_delimited_data(
    config: DelimitedReaderConfig,
    mut input: PipelineData,
    name: Span,
) -> Result<PipelineData, ShellError> {
    let metadata = input.take_metadata().map(|md| md.with_content_type(None));
    match input {
        PipelineData::Empty => Ok(PipelineData::empty()),
        PipelineData::Value(value, ..) => {
            let string = value.into_string()?;
            let byte_stream = ByteStream::read_string(string, name, Signals::empty());
            Ok(PipelineData::list_stream(
                from_delimited_stream(config, byte_stream, name)?,
                metadata,
            ))
        }
        PipelineData::ListStream(list_stream, _) => Err(ShellError::OnlySupportsThisInputType {
            exp_input_type: "string".into(),
            wrong_type: "list".into(),
            dst_span: name,
            src_span: list_stream.span(),
        }),
        PipelineData::ByteStream(byte_stream, ..) => Ok(PipelineData::list_stream(
            from_delimited_stream(config, byte_stream, name)?,
            metadata,
        )),
    }
}

pub fn trim_from_str(trim: Option<Value>) -> Result<Trim, ShellError> {
    match trim {
        Some(v) => {
            let span = v.span();
            match v {
                Value::String {val: item, ..} => match item.as_str() {

            "all" => Ok(Trim::All),
            "headers" => Ok(Trim::Headers),
            "fields" => Ok(Trim::Fields),
            "none" => Ok(Trim::None),
            _ => Err(ShellError::TypeMismatch {
                err_message:
                    "the only possible values for trim are 'all', 'headers', 'fields' and 'none'"
                        .into(),
                span,
            }),
                }
                _ => Ok(Trim::None),
            }
        }
        _ => Ok(Trim::None),
    }
}
