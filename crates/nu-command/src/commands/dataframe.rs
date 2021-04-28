use crate::prelude::*;
use nu_engine::WholeStreamCommand;
use nu_errors::ShellError;
use nu_protocol::{
    nu_dataframe::NuDataFrame, Primitive, Signature, SyntaxShape, UntaggedValue, Value,
};
use nu_source::Tagged;
use std::fs::File;
use std::path::PathBuf;

use polars::prelude::{CsvReader, SerReader};

pub struct Dataframe;

impl WholeStreamCommand for Dataframe {
    fn name(&self) -> &str {
        "dataframe"
    }

    fn usage(&self) -> &str {
        "Creates a dataframe from a csv file"
    }

    fn signature(&self) -> Signature {
        Signature::build("dataframe").named(
            "file",
            SyntaxShape::FilePath,
            "the file path to load values from",
            Some('f'),
        )
    }

    fn run(&self, args: CommandArgs) -> Result<OutputStream, ShellError> {
        load_dataframe(args)
    }

    fn examples(&self) -> Vec<Example> {
        vec![Example {
            description: "Takes a file name and creates a dataframe",
            example: "dataframe -f test.csv",
            result: None,
        }]
    }
}

// Creates a dataframe from either a file or a table.
// If both options are found, then an error is returned to the user.
// The InputStream can have a table and a dictionary as input variable.
fn load_dataframe(args: CommandArgs) -> Result<OutputStream, ShellError> {
    if args
        .call_info
        .args
        .named
        .as_ref()
        .map(|a| a.named.contains_key("file"))
        .is_some()
    {
        create_from_file(args)
    } else {
        let tag = args.call_info.name_tag.clone();
        let init =
            InputStream::one(UntaggedValue::Primitive(Primitive::Boolean(true)).into_value(&tag));

        Ok(init.to_output_stream())
    }
}

fn create_from_file(args: CommandArgs) -> Result<OutputStream, ShellError> {
    // Command Tag. This marks where the command is located and the name
    // of the command used
    let tag = args.call_info.name_tag.clone();

    // When the arguments get evaluated, the EvaluationContext is used
    // to mark the scope and other variables related to the input
    let args = args.evaluate_once()?;

    //println!("{:?}", args.args.call_info.args);
    //println!("{:?}", args.input.into_vec());

    // The flag file has priority over the input stream
    if let Some(value) = args.get_flag::<Value>("file")? {
        if let UntaggedValue::Primitive(Primitive::FilePath(path)) = value.value {
            // Needs more detail and arguments while loading the dataframe
            // Options:
            //  - has header
            //  - infer schema
            //  - delimiter
            //  - csv or parquet <- extracted from extension
            let csv_reader = match CsvReader::from_path(path) {
                Ok(df) => df,
                Err(e) => {
                    return Err(ShellError::labeled_error(
                        "Unable to parse file",
                        format!("error: {}", e),
                        &value.tag,
                    ))
                }
            };

            let df = csv_reader
                .infer_schema(None)
                .has_header(true)
                .finish()
                .expect("error");

            let nu_dataframe = NuDataFrame {
                dataframe: Some(df),
            };

            let init = InputStream::one(UntaggedValue::Dataframe(nu_dataframe).into_value(&tag));

            return Ok(init.to_output_stream());
        }
    }

    let init =
        InputStream::one(UntaggedValue::Primitive(Primitive::Boolean(true)).into_value(&tag));

    Ok(init.to_output_stream())
}
