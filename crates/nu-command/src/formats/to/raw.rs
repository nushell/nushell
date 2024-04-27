use nu_engine::command_prelude::*;

#[derive(Clone)]
pub struct ToRaw;

impl Command for ToRaw {
    fn name(&self) -> &str {
        "to raw"
    }

    fn signature(&self) -> Signature {
        Signature::build(self.name())
            .input_output_types(vec![
                (Type::String, Type::Any),
                (Type::Binary, Type::Any),
                (Type::List(Type::Any.into()), Type::Any),
            ])
            .category(Category::Formats)
    }

    fn usage(&self) -> &str {
        "Convert a stream to raw input."
    }

    fn extra_usage(&self) -> &str {
        r#"
Renders input of string or binary value(s) as raw input data, the same as if it
were received as the output of an external command.

This is helpful when trying to print binary data as the output of a script, and
it also preserves the streaming characteristics of the input, unlike `bytes
collect`, so it can be used in situations where collecting all of the data at
once at first would be undesirable.
"#
        .trim()
    }

    fn search_terms(&self) -> Vec<&str> {
        vec!["binary", "bytes"]
    }

    fn examples(&self) -> Vec<Example> {
        vec![
            Example {
                example: "0x[aa bb cc dd] | to raw | print",
                description: "Print raw bytes to stdout",
                result: None,
            },
            Example {
                example: "[0x[aa bb] 0x[cc dd]] | to raw | print",
                description: "Print multiple strings of raw bytes to stdout",
                result: None,
            },
            Example {
                example: "[0x[ff03] foo 0x[ff03] bar] | to raw | print",
                description: "Print mixed text and binary data to stdout",
                result: None,
            },
            Example {
                example: "seq 1 10 | each { into binary } | to raw | save --raw test.bin",
                description: "Save a stream of numbers as binary to a file, without collecting in-memory first",
                result: None,
            },
        ]
    }

    fn run(
        &self,
        engine_state: &EngineState,
        _stack: &mut Stack,
        call: &Call,
        input: PipelineData,
    ) -> Result<PipelineData, ShellError> {
        let metadata = input.metadata();
        Ok(PipelineData::ExternalStream {
            stdout: Some(input.to_raw_stream(&engine_state.ctrlc, call.head)),
            stderr: None,
            exit_code: None,
            span: call.head,
            metadata,
            trim_end_newline: false,
        })
    }
}
