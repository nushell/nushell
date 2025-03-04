use nu_engine::command_prelude::*;

#[derive(Clone)]
pub struct MailClear;

impl Command for MailClear {
    fn name(&self) -> &str {
        "mail clear"
    }

    fn description(&self) -> &str {
        "Clear mailbox."
    }

    fn extra_description(&self) -> &str {
        r#"
This command removes all messages in the mailbox of the current job.
If a message is received while this command is executing, it may also be discarded.
"#
    }

    fn signature(&self) -> nu_protocol::Signature {
        Signature::build("mail flush")
            .category(Category::Experimental)
            .input_output_types(vec![(Type::Nothing, Type::Nothing)])
            .allow_variants_without_examples(true)
    }

    fn search_terms(&self) -> Vec<&str> {
        vec![]
    }

    fn run(
        &self,
        engine_state: &EngineState,
        _stack: &mut Stack,
        call: &Call,
        _input: PipelineData,
    ) -> Result<PipelineData, ShellError> {
        let mut mailbox = engine_state
            .current_job
            .mailbox
            .lock()
            .expect("failed to acquire lock");

        mailbox.clear();

        Ok(Value::nothing(call.head).into_pipeline_data())
    }

    fn examples(&self) -> Vec<Example> {
        vec![Example {
            example: "let id = job spawn { mail recv | save sent.txt }; 'hi' | mail send $id",
            description: "Send a message to a newly spawned job",
            result: None,
        }]
    }
}
