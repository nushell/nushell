use std::fs::File;
use std::path::PathBuf;

use crate::prelude::*;
use nu_engine::WholeStreamCommand;
use nu_errors::ShellError;
use nu_protocol::Primitive;
use nu_protocol::Value;
use nu_protocol::{dataframe::PolarsData, Signature, SyntaxShape, UntaggedValue};

use polars::prelude::{CsvWriter, SerWriter};

use nu_source::Tagged;

use super::utils::parse_polars_error;
pub struct DataFrame;

impl WholeStreamCommand for DataFrame {
    fn name(&self) -> &str {
        "pls to_csv"
    }

    fn usage(&self) -> &str {
        "Saves dataframe to csv file"
    }

    fn signature(&self) -> Signature {
        Signature::build("pls to_csv")
            .required("file", SyntaxShape::FilePath, "file path to save dataframe")
            .named(
                "delimiter",
                SyntaxShape::String,
                "file delimiter character",
                Some('d'),
            )
            .switch("no_header", "Indicates if file doesn't have header", None)
    }

    fn run(&self, args: CommandArgs) -> Result<OutputStream, ShellError> {
        command(args)
    }

    fn examples(&self) -> Vec<Example> {
        vec![
            Example {
                description: "Saves dataframe to csv file",
                example: "[[a b]; [1 2] [3 4]] | pls convert | pls to_csv test.csv",
                result: None,
            },
            Example {
                description: "Saves dataframe to csv file using other delimiter",
                example: "[[a b]; [1 2] [3 4]] | pls convert | pls to_csv test.csv -d '|'",
                result: None,
            },
        ]
    }
}

fn command(args: CommandArgs) -> Result<OutputStream, ShellError> {
    let tag = args.call_info.name_tag.clone();
    let mut args = args.evaluate_once()?;
    let file_name: Tagged<PathBuf> = args.req(0)?;
    let delimiter: Option<Tagged<String>> = args.get_flag("delimiter")?;
    let no_header: bool = args.has_flag("no_header");

    let mut df = args
        .input
        .next()
        .and_then(|value| match value.value {
            UntaggedValue::DataFrame(PolarsData::EagerDataFrame(df)) => Some(df),
            _ => None,
        })
        .ok_or(ShellError::labeled_error(
            "No input received",
            "missing dataframe input from stream",
            &tag.span,
        ))?;

    let mut file = File::create(&file_name.item).map_err(|e| {
        ShellError::labeled_error(
            "Error with file name",
            format!("{}", e),
            &file_name.tag.span,
        )
    })?;

    let writer = CsvWriter::new(&mut file);

    let writer = if no_header {
        writer.has_headers(false)
    } else {
        writer.has_headers(true)
    };

    let writer = match delimiter {
        None => writer,
        Some(d) => {
            if d.item.len() != 1 {
                return Err(ShellError::labeled_error(
                    "Incorrect delimiter",
                    "Delimiter has to be one char",
                    &d.tag,
                ));
            } else {
                let delimiter = match d.item.chars().nth(0) {
                    Some(d) => d as u8,
                    None => unreachable!(),
                };

                writer.with_delimiter(delimiter)
            }
        }
    };

    writer
        .finish(df.as_mut())
        .map_err(|e| parse_polars_error::<&str>(&e, &file_name.tag.span, None))?;

    let tagged_value = Value {
        value: UntaggedValue::Primitive(Primitive::String(format!(
            "saved {}",
            &file_name.item.to_str().expect("csv file")
        ))),
        tag: Tag::unknown(),
    };

    Ok(InputStream::one(tagged_value).to_output_stream())
}
