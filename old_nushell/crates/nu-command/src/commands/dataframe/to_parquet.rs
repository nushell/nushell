use std::fs::File;
use std::path::PathBuf;

use crate::prelude::*;
use nu_engine::WholeStreamCommand;
use nu_errors::ShellError;
use nu_protocol::dataframe::NuDataFrame;
use nu_protocol::{Primitive, Signature, SyntaxShape, UntaggedValue, Value};

use polars::prelude::ParquetWriter;

use nu_source::Tagged;

use super::utils::parse_polars_error;
pub struct DataFrame;

impl WholeStreamCommand for DataFrame {
    fn name(&self) -> &str {
        "dataframe to-parquet"
    }

    fn usage(&self) -> &str {
        "[DataFrame] Saves dataframe to parquet file"
    }

    fn signature(&self) -> Signature {
        Signature::build("dataframe to-parquet").required(
            "file",
            SyntaxShape::FilePath,
            "file path to save dataframe",
        )
    }

    fn run(&self, args: CommandArgs) -> Result<OutputStream, ShellError> {
        command(args)
    }

    fn examples(&self) -> Vec<Example> {
        vec![Example {
            description: "Saves dataframe to parquet file",
            example: "[[a b]; [1 2] [3 4]] | dataframe to-df | dataframe to-parquet test.parquet",
            result: None,
        }]
    }
}

fn command(mut args: CommandArgs) -> Result<OutputStream, ShellError> {
    let tag = args.call_info.name_tag.clone();
    let file_name: Tagged<PathBuf> = args.req(0)?;

    let (df, _) = NuDataFrame::try_from_stream(&mut args.input, &tag.span)?;

    let file = File::create(&file_name.item).map_err(|e| {
        ShellError::labeled_error("Error with file name", e.to_string(), &file_name.tag.span)
    })?;

    ParquetWriter::new(file)
        .finish(df.as_ref())
        .map_err(|e| parse_polars_error::<&str>(&e, &file_name.tag.span, None))?;

    let tagged_value = Value {
        value: UntaggedValue::Primitive(Primitive::String(format!(
            "saved {}",
            &file_name.item.to_str().expect("parquet file")
        ))),
        tag: Tag::unknown(),
    };

    Ok(InputStream::one(tagged_value).into_output_stream())
}
