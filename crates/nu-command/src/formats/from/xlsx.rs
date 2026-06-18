use std::io::Cursor;

use calamine::*;

use super::sheets::{collect_binary, from_sheets};
use nu_engine::command_prelude::*;

#[derive(Clone)]
pub struct FromXlsx;

impl Command for FromXlsx {
    fn name(&self) -> &str {
        "from xlsx"
    }

    fn signature(&self) -> Signature {
        Signature::build("from xlsx")
            .input_output_types(vec![(Type::Binary, Type::record())])
            .allow_variants_without_examples(true)
            .named(
                "sheets",
                SyntaxShape::List(Box::new(SyntaxShape::String)),
                "Only convert specified sheets.",
                Some('s'),
            )
            .category(Category::Formats)
    }

    fn description(&self) -> &str {
        "Parse binary Excel(.xlsx) data and create table."
    }

    fn run(
        &self,
        engine_state: &EngineState,
        stack: &mut Stack,
        call: &Call,
        mut input: PipelineData,
    ) -> Result<PipelineData, ShellError> {
        let head = call.head;
        let sel_sheets = call
            .get_flag::<Vec<String>>(engine_state, stack, "sheets")?
            .unwrap_or_default();
        let metadata = input.take_metadata().map(|md| md.with_content_type(None));

        let input_span = input.span().unwrap_or(head);
        let bytes = collect_binary(input, head)?;
        let buf: Cursor<Vec<u8>> = Cursor::new(bytes);
        let sheets = Sheets::Xlsx(Xlsx::new(buf).map_err(|_| ShellError::UnsupportedInput {
            msg: "Could not load XLSX file".to_string(),
            input: "value originates from here".into(),
            msg_span: head,
            input_span,
        })?);

        from_sheets(sheets, sel_sheets, input_span, head).map(|pd| pd.set_metadata(metadata))
    }

    fn examples(&self) -> Vec<Example<'_>> {
        vec![
            Example {
                description: "Convert binary .xlsx data to a table.",
                example: "open --raw test.xlsx | from xlsx",
                result: None,
            },
            Example {
                description: "Convert binary .xlsx data to a table, specifying the tables.",
                example: "open --raw test.xlsx | from xlsx --sheets [Spreadsheet1]",
                result: None,
            },
            Example {
                description: "Convert binary .xlsx data to a table, specifying the tables and specifying no header row.",
                example: "open --raw test.xlsx | from xlsx --sheets [Spreadsheet1] --header-row null",
                result: None,
            },
        ]
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_examples() -> nu_test_support::Result {
        nu_test_support::test().examples(FromXlsx)
    }
}
