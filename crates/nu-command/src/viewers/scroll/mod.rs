mod pager;

use nu_engine::{get_columns, CallExt};
use nu_protocol::{
    ast::{Call, PathMember},
    engine::{Command, EngineState, Stack},
    Category, Example, PipelineData, ShellError, Signature, Span, SyntaxShape, Value,
};

#[derive(Clone)]
pub struct Scroll;

impl Command for Scroll {
    fn name(&self) -> &str {
        "scroll"
    }

    fn usage(&self) -> &str {
        "Scroll acts as a simple table pager, just like `less` does for text"
    }

    fn signature(&self) -> nu_protocol::Signature {
        // todo: Fix error message when it's empty
        // if we set h i short flags it panics????

        Signature::build("tabless")
            .named(
                "head",
                SyntaxShape::Boolean,
                "Setting it to false makes it doesn't show column headers",
                None,
            )
            .switch("index", "A flag to show a index beside the rows", Some('i'))
            .switch(
                "reverse",
                "Makes it start from the end. (like `more`)",
                Some('r'),
            )
            .category(Category::Viewers)
    }

    fn extra_usage(&self) -> &str {
        ""
    }

    fn run(
        &self,
        engine_state: &EngineState,
        stack: &mut Stack,
        call: &Call,
        input: PipelineData,
    ) -> Result<PipelineData, ShellError> {
        let show_head: bool = call.get_flag(engine_state, stack, "head")?.unwrap_or(true);
        let show_index: bool = call.has_flag("index");
        let is_reverse: bool = call.has_flag("reverse");

        let ctrlc = engine_state.ctrlc.clone();
        let config = engine_state.get_config();

        let (columns, data) = collect_pipeline(input);

        let _ = pager::handler(
            &columns, &data, config, ctrlc, show_index, show_head, is_reverse,
        );

        Ok(PipelineData::Value(Value::default(), None))
    }

    fn examples(&self) -> Vec<Example> {
        vec![]
    }
}

fn collect_pipeline(input: PipelineData) -> (Vec<String>, Vec<Vec<Value>>) {
    match input {
        PipelineData::Value(value, ..) => collect_input(value),
        PipelineData::ListStream(mut stream, ..) => {
            let mut records = vec![];
            for item in stream.by_ref() {
                records.push(item);
            }

            let cols = get_columns(&records);
            let data = convert_records_to_dataset(&cols, records);

            (cols, data)
        }
        PipelineData::ExternalStream { .. } => (Vec::new(), Vec::new()),
    }
}

pub(crate) fn collect_input(value: Value) -> (Vec<String>, Vec<Vec<Value>>) {
    match value {
        Value::Record { cols, vals, .. } => (cols, vec![vals]),
        Value::List { vals, .. } => {
            let columns = get_columns(&vals);
            let data = convert_records_to_dataset(&columns, vals);

            (columns, data)
        }
        Value::String { val, span } => {
            let lines = val
                .lines()
                .map(|line| Value::String {
                    val: line.to_string(),
                    span,
                })
                .map(|val| vec![val])
                .collect();

            (Vec::new(), lines)
        }
        value => (Vec::new(), vec![vec![value]]),
    }
}

fn convert_records_to_dataset(cols: &Vec<String>, records: Vec<Value>) -> Vec<Vec<Value>> {
    if !cols.is_empty() {
        create_table_for_record(cols, &records)
    } else if cols.is_empty() && records.is_empty() {
        vec![]
    } else if cols.len() == records.len() {
        vec![records]
    } else {
        // I am not sure whether it's good to return records as its length LIKELY will not match columns,
        // which makes no scense......
        //
        // BUT...
        // we can represent it as a list; which we do

        records.into_iter().map(|record| vec![record]).collect()
    }
}

fn create_table_for_record(headers: &[String], items: &[Value]) -> Vec<Vec<Value>> {
    let mut data = vec![Vec::new(); items.len()];

    for (i, item) in items.iter().enumerate() {
        let row = record_create_row(headers, item);
        data[i] = row;
    }

    data
}

fn record_create_row(headers: &[String], item: &Value) -> Vec<Value> {
    let mut rows = vec![Value::default(); headers.len()];

    for (i, header) in headers.iter().enumerate() {
        let value = record_lookup_value(item, header);
        rows[i] = value;
    }

    rows
}

fn record_lookup_value(item: &Value, header: &str) -> Value {
    match item {
        Value::Record { .. } => {
            let path = PathMember::String {
                val: header.to_owned(),
                span: Span::unknown(),
            };

            let value = item.clone().follow_cell_path(&[path], false);
            match value {
                Ok(value) => value,
                Err(_) => item.clone(),
            }
        }
        item => item.clone(),
    }
}
