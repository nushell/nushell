use std::path::PathBuf;

use crate::prelude::*;
use nu_engine::WholeStreamCommand;
use nu_errors::ShellError;
use nu_protocol::{dataframe::NuDataFrame, Signature, SyntaxShape, UntaggedValue};

use nu_source::Tagged;
use polars::prelude::{CsvReader, SerReader};

pub struct Dataframe;

impl WholeStreamCommand for Dataframe {
    fn name(&self) -> &str {
        "dataframe load"
    }

    fn usage(&self) -> &str {
        "Loads dataframe form csv or parquet file"
    }

    fn signature(&self) -> Signature {
        Signature::build("dataframe load").required(
            "file",
            SyntaxShape::FilePath,
            "the file path to load values from",
        )
    }

    fn run(&self, args: CommandArgs) -> Result<OutputStream, ShellError> {
        create_from_file(args)
    }

    fn examples(&self) -> Vec<Example> {
        vec![Example {
            description: "Takes a file name and creates a dataframe",
            example: "dataframe load test.csv",
            result: None,
        }]
    }
}

fn create_from_file(args: CommandArgs) -> Result<OutputStream, ShellError> {
    let tag = args.call_info.name_tag.clone();
    let args = args.evaluate_once()?;
    let file: Tagged<PathBuf> = args.req(0)?;

    // Needs more detail and arguments while loading the dataframe
    // Options:
    //  - has header
    //  - infer schema
    //  - delimiter
    //  - csv or parquet <- extracted from extension
    let csv_reader = match CsvReader::from_path(&file.item) {
        Ok(csv_reader) => csv_reader,
        Err(e) => {
            return Err(ShellError::labeled_error(
                "Unable to parse file",
                format!("{}", e),
                &file.tag,
            ))
        }
    };

    let df = match csv_reader.infer_schema(None).has_header(true).finish() {
        Ok(csv_reader) => csv_reader,
        Err(e) => {
            return Err(ShellError::labeled_error(
                "Error while parsing dataframe",
                format!("{}", e),
                &file.tag,
            ))
        }
    };

    let file_name = match file.item.into_os_string().into_string() {
        Ok(name) => name,
        Err(e) => {
            return Err(ShellError::labeled_error(
                "Error with file name",
                format!("{:?}", e),
                &file.tag,
            ))
        }
    };

    let nu_dataframe = NuDataFrame {
        dataframe: Some(df),
        name: file_name,
    };

    let init = InputStream::one(UntaggedValue::Dataframe(nu_dataframe).into_value(&tag));

    Ok(init.to_output_stream())
}
