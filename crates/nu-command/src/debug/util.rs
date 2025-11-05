use nu_protocol::{DataSource, IntoValue, PipelineData, PipelineMetadata, Record, Span, Value};
use std::path::PathBuf;

pub fn extend_record_with_metadata(
    mut record: Record,
    metadata: Option<&PipelineMetadata>,
    head: Span,
) -> Record {
    if let Some(PipelineMetadata {
        data_source,
        content_type,
        custom,
    }) = metadata
    {
        match data_source {
            DataSource::Ls => record.push("source", Value::string("ls", head)),
            DataSource::HtmlThemes => {
                record.push("source", Value::string("into html --list", head))
            }
            DataSource::FilePath(path) => record.push(
                "source",
                Value::string(path.to_string_lossy().to_string(), head),
            ),
            DataSource::None => {}
        }
        if let Some(content_type) = content_type {
            record.push("content_type", Value::string(content_type, head));
        }
        for (key, value) in custom {
            record.push(key, value.clone());
        }
    };

    record
}

pub fn parse_metadata_from_record(record: &Record) -> PipelineMetadata {
    let mut metadata = PipelineMetadata::default();
    let mut custom = Record::new();

    for (key, value) in record {
        match key.as_str() {
            "source" => {
                if let Ok(s) = value.as_str() {
                    metadata.data_source = match s {
                        "ls" => DataSource::Ls,
                        "into html --list" => DataSource::HtmlThemes,
                        _ => DataSource::FilePath(PathBuf::from(s)),
                    };
                }
            }
            "content_type" => {
                if !value.is_nothing()
                    && let Ok(s) = value.as_str()
                {
                    metadata.content_type = Some(s.to_string());
                }
            }
            "span" => {
                // Skip span field - it's metadata about the value, not pipeline metadata
            }
            _ => {
                // Any other field goes into custom metadata
                custom.push(key.clone(), value.clone());
            }
        }
    }

    metadata.custom = custom;
    metadata
}

pub fn build_metadata_record(pipeline: &PipelineData, head: Span) -> Record {
    let mut record = Record::new();
    if let Some(span) = pipeline.span() {
        record.insert("span", span.into_value(head));
    }
    extend_record_with_metadata(record, pipeline.metadata().as_ref(), head)
}
