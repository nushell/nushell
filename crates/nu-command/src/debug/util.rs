use nu_protocol::{DataSource, IntoValue, PipelineData, PipelineMetadata, Record, Span, Value};

pub fn extend_record_with_metadata(
    mut record: Record,
    metadata: Option<&PipelineMetadata>,
    head: Span,
) -> Record {
    if let Some(PipelineMetadata {
        data_source,
        content_type,
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
    };

    record
}

pub fn build_metadata_record(pipeline: &PipelineData, head: Span) -> Record {
    let mut record = Record::new();
    if let Some(span) = pipeline.span() {
        record.insert("span", span.into_value(head));
    }
    extend_record_with_metadata(record, pipeline.metadata().as_ref(), head)
}
