use std::path::PathBuf;

use crate::prelude::*;
use nu_engine::WholeStreamCommand;
use nu_errors::ShellError;
use nu_protocol::{
    hir::NamedValue, nu_dataframe::NuDataFrame, Signature, SyntaxShape, UntaggedValue,
};

use nu_source::Tagged;
use polars::prelude::{CsvReader, SerReader};

pub struct Dataframe;

#[derive(Deserialize)]
pub struct OpenArgs {
    file: Tagged<PathBuf>,
}

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
        vec![
            Example {
                description: "Takes a file name and creates a dataframe",
                example: "dataframe -f test.csv",
                result: None,
            },
            Example {
                description: "Takes an input stream and converts it to a dataframe",
                example: "echo [[a b];[1 2] [3 4]] | dataframe",
                result: None,
            },
        ]
    }
}

// Creates a dataframe from either a file or a table.
// If both options are found, then an error is returned to the user.
// The InputStream can have a table and a dictionary as input variable.
fn load_dataframe(args: CommandArgs) -> Result<OutputStream, ShellError> {
    // The file has priority over stream input
    if let Some(NamedValue::Value(_, _)) = args
        .call_info()
        .args
        .named
        .as_ref()
        .map(|named| named.named.get("file"))
        .flatten()
    {
        return create_from_file(args);
    }

    create_from_input(args)
}

fn create_from_file(args: CommandArgs) -> Result<OutputStream, ShellError> {
    // Command Tag. This marks where the command is located and the name
    // of the command used
    let tag = args.call_info.name_tag.clone();

    // Parsing the arguments that the function uses
    let (OpenArgs { file }, _) = args.process()?;

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

    return Ok(init.to_output_stream());
}

fn create_from_input(args: CommandArgs) -> Result<OutputStream, ShellError> {
    let tag = args.call_info.name_tag.clone();
    let args = args.evaluate_once()?;
    let df = NuDataFrame::try_from_iter(args.input, &tag)?;
    let init = InputStream::one(UntaggedValue::Dataframe(df).into_value(&tag));

    Ok(init.to_output_stream())
}
