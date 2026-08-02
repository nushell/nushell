use calamine::{Reader, Sheets, Xlsx};

use super::sheets::{collect_binary, common_sheets_signature, from_sheets};
use nu_engine::command_prelude::*;

#[derive(Clone)]
pub struct FromXlsx;

impl Command for FromXlsx {
    fn name(&self) -> &str {
        "from xlsx"
    }

    fn signature(&self) -> Signature {
        common_sheets_signature("from xlsx")
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
        let metadata = input.take_metadata().map(|md| md.with_content_type(None));

        let input_span = input.span().unwrap_or(head);
        let reader = collect_binary(input, head)?;
        let xlsx = Xlsx::new(reader).map_err(|_| ShellError::UnsupportedInput {
            msg: "Could not load XLSX file".to_string(),
            input: "value originates from here".into(),
            msg_span: head,
            input_span,
        })?;
        let sheets = Sheets::Xlsx(xlsx);

        from_sheets(sheets, input_span, engine_state, stack, call)
            .map(|pd| pd.set_metadata(metadata))
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
                example: "open --raw test.xlsx | from xlsx --sheets [Spreadsheet1] --noheaders",
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
