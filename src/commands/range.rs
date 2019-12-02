use crate::commands::WholeStreamCommand;
use crate::errors::ShellError;
use crate::context::CommandRegistry;
use crate::prelude::*;
use nu_source::Tagged;

#[derive(Deserialize)]
struct RangeArgs {
    area: Tagged<String>,
}

pub struct Range;

impl WholeStreamCommand for Range {
    fn name(&self) -> &str {
        "range"
    }

    fn signature(&self) -> Signature {
        Signature::build("range").required(
            "rows ",
            SyntaxShape::Any,
            "range of rows to return: Eg) 4..7 (=> from 4 to 7)",
        )
    }

    fn usage(&self) -> &str {
        "Return only the selected rows"
    }

    fn run(
        &self,
        args: CommandArgs,
        registry: &CommandRegistry,
    ) -> Result<OutputStream, ShellError> {
        args.process(registry, range)?.run()
    }
}

fn range(
    RangeArgs { area: rows }: RangeArgs,
    RunnableContext { input, name, .. }: RunnableContext,
) -> Result<OutputStream, ShellError> { 
    match rows.item.find(".") {
        Some(value) => {
            let (first, last) = rows.item.split_at(value);
            let first = match first.parse::<u64>() {
                Ok(postion) => postion,
                Err(_) => {
                    if first == "" {
                        0
                    } else {
                        return Err(ShellError::labeled_error(
                            "no correct start of range",
                            "'from' needs to be an Integer or empty",
                            name,
                        ));
                    }
                },
            };
            let last = match last.trim_start_matches(".").parse::<u64>() {
                Ok(postion) => postion,
                Err(_) => {
                    if last == ".." {
                        std::u64::MAX
                    } else {
                        return Err(ShellError::labeled_error(
                            "no correct end of range",
                            "'to' needs to be an Integer or empty",
                            name,
                        ));
                    }
                },
            };
            return Ok(OutputStream::from_input(
                input.values.skip(first).take(last-first+1),
            ));
        },
        None => {
            return Err(ShellError::labeled_error(
                "No correct formated range found",
                "format: <from>..<to>",
                name,
            ));
        }
    }
}