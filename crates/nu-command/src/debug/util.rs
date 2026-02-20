use nu_protocol::{DataSource, IntoValue, PipelineData, PipelineMetadata, Record, Span, Value};
use std::path::PathBuf;

pub fn extend_record_with_metadata(
    mut record: Record,
    metadata: Option<&PipelineMetadata>,
    head: Span,
) -> Record {
    if let Some(PipelineMetadata {
        data_source,
        path_columns,
        content_type,
        custom,
    }) = metadata
    {
        match data_source {
            #[allow(deprecated)]
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
        if !path_columns.is_empty() {
            let path_columns = path_columns
                .iter()
                .map(|col| Value::string(col, head))
                .collect();
            record.push("path_columns", Value::list(path_columns, head));
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
                        #[allow(deprecated)]
                        "ls" => DataSource::Ls,
                        "into html --list" => DataSource::HtmlThemes,
                        _ => DataSource::FilePath(PathBuf::from(s)),
                    };
                }
            }
            "path_columns" => {
                let path_columns: Option<Vec<String>> = value.as_list().ok().and_then(|list| {
                    list.iter()
                        .map(|value| value.as_str().map(String::from).ok())
                        .collect()
                });
                if let Some(path_columns) = path_columns {
                    metadata.path_columns = path_columns;
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
