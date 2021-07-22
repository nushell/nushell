use crate::prelude::*;
use nu_engine::WholeStreamCommand;
use nu_errors::ShellError;
use nu_protocol::{dataframe::NuDataFrame, Signature, SyntaxShape};
use nu_source::Tagged;

pub struct DataFrame;

impl WholeStreamCommand for DataFrame {
    fn name(&self) -> &str {
        "dataframe rename"
    }

    fn usage(&self) -> &str {
        "[Series] Renames a series"
    }

    fn signature(&self) -> Signature {
        Signature::build("dataframe rename").required(
            "name",
            SyntaxShape::String,
            "new series name",
        )
    }

    fn run(&self, args: CommandArgs) -> Result<OutputStream, ShellError> {
        command(args)
    }

    fn examples(&self) -> Vec<Example> {
        vec![Example {
            description: "Renames a series",
            example: "[5 6 7 8] | dataframe to-df | dataframe rename new_name",
            result: None,
        }]
    }
}

fn command(mut args: CommandArgs) -> Result<OutputStream, ShellError> {
    let tag = args.call_info.name_tag.clone();
    let name: Tagged<String> = args.req(0)?;

    let (df, df_tag) = NuDataFrame::try_from_stream(&mut args.input, &tag.span)?;

    let mut series = df.as_series(&df_tag.span)?;

    series.rename(name.item.as_ref());

    let df = NuDataFrame::try_from_series(vec![series], &tag.span)?;
    Ok(OutputStream::one(df.into_value(df_tag)))
}
