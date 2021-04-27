use crate::prelude::*;
use nu_engine::WholeStreamCommand;
use nu_errors::ShellError;
use nu_protocol::{nu_dataframe::NuDataFrame, Signature, SyntaxShape, UntaggedValue};
use nu_source::Tagged;
use std::fs::File;
use std::path::PathBuf;

use polars::prelude::{CsvReader, SerReader};

#[derive(Deserialize)]
pub struct OpenArgs {
    path: Tagged<PathBuf>,
}

pub struct Dataframe;

impl WholeStreamCommand for Dataframe {
    fn name(&self) -> &str {
        "dataframe"
    }

    fn usage(&self) -> &str {
        "Creates a dataframe from a csv file"
    }

    fn signature(&self) -> Signature {
        Signature::build("dataframe").required(
            "file",
            SyntaxShape::FilePath,
            "the file path to load values from",
        )
    }

    fn run(&self, args: CommandArgs) -> Result<OutputStream, ShellError> {
        load_dataframe(args)
    }

    fn examples(&self) -> Vec<Example> {
        vec![Example {
            description: "Takes a csv file name",
            example: "dataframe test.csv",
            result: None,
        }]
    }
}

fn load_dataframe(args: CommandArgs) -> Result<OutputStream, ShellError> {
    let tag = args.call_info.name_tag.clone();

    let (OpenArgs { path }, _) = args.process()?;

    let file = match File::open(path.item) {
        Ok(file) => file,
        Err(_) => {
            return Err(ShellError::labeled_error(
                "Issue reading file",
                "invalid file",
                &path.tag,
            ))
        }
    };

    let df = CsvReader::new(file)
        .infer_schema(None)
        .has_header(true)
        .finish()
        .expect("Error reading file");

    println!("{}", df);

    let nu_dataframe = NuDataFrame {
        dataframe: Some(df),
    };

    let init = InputStream::one(UntaggedValue::Dataframe(nu_dataframe).into_value(&tag));

    Ok(init.to_output_stream())
}
