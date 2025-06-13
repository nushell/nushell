use nu_engine::{command_prelude::*, get_full_help};

use super::get::run_get;
use super::post::run_post;

#[derive(Clone)]
pub struct Http;

impl Command for Http {
    fn name(&self) -> &str {
        "http"
    }

    fn signature(&self) -> Signature {
        Signature::build("http")
            .input_output_types(vec![(Type::Nothing, Type::Any)])
            // common to get more than help. Get by default
            .optional(
                "URL",
                SyntaxShape::String,
                "The URL to fetch the contents from.",
            )
            // post
            .optional(
                "data",
                SyntaxShape::Any,
                "The contents of the post body. Required unless part of a pipeline.",
            )
            .named(
                "content-type",
                SyntaxShape::Any,
                "the MIME type of content to post",
                Some('t'),
            )
            // common
            .named(
                "user",
                SyntaxShape::Any,
                "the username when authenticating",
                Some('u'),
            )
            .named(
                "password",
                SyntaxShape::Any,
                "the password when authenticating",
                Some('p'),
            )
            .named(
                "max-time",
                SyntaxShape::Duration,
                "max duration before timeout occurs",
                Some('m'),
            )
            .named(
                "headers",
                SyntaxShape::Any,
                "custom headers you want to add ",
                Some('H'),
            )
            .switch(
                "raw",
                "fetch contents as text rather than a table",
                Some('r'),
            )
            .switch(
                "insecure",
                "allow insecure server connections when using SSL",
                Some('k'),
            )
            .switch(
                "full",
                "returns the full response instead of only the body",
                Some('f'),
            )
            .switch(
                "allow-errors",
                "do not fail if the server returns an error code",
                Some('e'),
            )
            .named(
                "redirect-mode",
                SyntaxShape::String,
                "What to do when encountering redirects. Default: 'follow'. Valid options: 'follow' ('f'), 'manual' ('m'), 'error' ('e').",
                Some('R')
            )
            .category(Category::Network)
    }

    fn description(&self) -> &str {
        "Various commands for working with http methods."
    }

    fn extra_description(&self) -> &str {
        "Without a subcommand but with a URL provided, it performs a GET request by default or a POST request if data is provided. You can use one of the following subcommands. Using this command as-is will only display this help message."
    }

    fn search_terms(&self) -> Vec<&str> {
        vec![
            "network", "fetch", "pull", "request", "download", "curl", "wget",
        ]
    }

    fn run(
        &self,
        engine_state: &EngineState,
        stack: &mut Stack,
        call: &Call,
        input: PipelineData,
    ) -> Result<PipelineData, ShellError> {
        let url = call.opt::<Value>(engine_state, stack, 0)?;
        let data = call.opt::<Value>(engine_state, stack, 1)?;
        match (url.is_some(), data.is_some()) {
            (true, true) => run_post(engine_state, stack, call, input),
            (true, false) => run_get(engine_state, stack, call, input),
            (false, true) => Err(ShellError::NushellFailed {
                msg: (String::from("Default verb is get with a payload. Impossible state")),
            }),
            (false, false) => Ok(Value::string(
                get_full_help(self, engine_state, stack),
                call.head,
            )
            .into_pipeline_data()),
        }
    }

    fn examples(&self) -> Vec<Example> {
        vec![
            Example {
                description: "Get content from example.com with default verb",
                example: "http https://www.example.com",
                result: None,
            },
            Example {
                description: "Post content to example.com with default verb",
                example: "http https://www.example.com 'body'",
                result: None,
            },
            Example {
                description: "Get content from example.com with explicit verb",
                example: "http get https://www.example.com",
                result: None,
            },
        ]
    }
}
