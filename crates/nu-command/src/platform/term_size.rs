use nu_protocol::ast::Call;
use nu_protocol::engine::{Command, EngineState, Stack};
use nu_protocol::{Category, Example, IntoPipelineData, PipelineData, Signature, Span, Value};
use terminal_size::{terminal_size, Height, Width};

#[derive(Clone)]
pub struct TermSize;

impl Command for TermSize {
    fn name(&self) -> &str {
        "term size"
    }

    fn usage(&self) -> &str {
        "Returns the terminal size"
    }

    fn signature(&self) -> Signature {
        Signature::build("term size")
            .switch(
                "columns",
                "Report only the width of the terminal",
                Some('c'),
            )
            .switch("rows", "Report only the height of the terminal", Some('r'))
            .category(Category::Platform)
    }

    fn examples(&self) -> Vec<Example> {
        vec![
            Example {
                description: "Return the width height of the terminal",
                example: "term size",
                result: None,
            },
            Example {
                description: "Return the width (columns) of the terminal",
                example: "term size -c",
                result: None,
            },
            Example {
                description: "Return the height (rows) of the terminal",
                example: "term size -r",
                result: None,
            },
        ]
    }

    fn run(
        &self,
        _engine_state: &EngineState,
        _stack: &mut Stack,
        call: &Call,
        _input: PipelineData,
    ) -> Result<nu_protocol::PipelineData, nu_protocol::ShellError> {
        let head = call.head;
        let wide = call.has_flag("columns");
        let tall = call.has_flag("rows");

        let (cols, rows) = match terminal_size() {
            Some((w, h)) => (Width(w.0), Height(h.0)),
            None => (Width(0), Height(0)),
        };

        Ok((match (wide, tall) {
            (true, false) => Value::Record {
                cols: vec!["columns".into()],
                vals: vec![Value::Int {
                    val: cols.0 as i64,
                    span: Span::test_data(),
                }],
                span: head,
            },
            (true, true) => Value::Record {
                cols: vec!["columns".into(), "rows".into()],
                vals: vec![
                    Value::Int {
                        val: cols.0 as i64,
                        span: Span::test_data(),
                    },
                    Value::Int {
                        val: rows.0 as i64,
                        span: Span::test_data(),
                    },
                ],
                span: head,
            },
            (false, true) => Value::Record {
                cols: vec!["rows".into()],
                vals: vec![Value::Int {
                    val: rows.0 as i64,
                    span: Span::test_data(),
                }],
                span: head,
            },
            (false, false) => Value::Record {
                cols: vec!["columns".into(), "rows".into()],
                vals: vec![
                    Value::Int {
                        val: cols.0 as i64,
                        span: Span::test_data(),
                    },
                    Value::Int {
                        val: rows.0 as i64,
                        span: Span::test_data(),
                    },
                ],
                span: head,
            },
        })
        .into_pipeline_data())
    }
}
