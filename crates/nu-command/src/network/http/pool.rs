use crate::network::http::client::{
    RedirectMode, RequestFlags, RequestMetadata, expand_unix_socket_path, http_client,
    http_client_pool, http_parse_redirect_mode, request_add_authorization_header,
    request_add_custom_headers, request_handle_response, request_set_timeout, send_request_no_body,
};
use crate::network::http::client::{add_unix_socket_flag, reset_http_client_pool};
use nu_engine::command_prelude::*;

#[derive(Clone)]
pub struct HttpPool;

impl Command for HttpOptions {
    fn name(&self) -> &str {
        "http pool"
    }

    fn signature(&self) -> Signature {
        let sig = Signature::build("http pool")
            .input_output_types(vec![(Type::Nothing, Type::Any)])
            .allow_variants_without_examples(true)
            .switch(
                "insecure",
                "allow insecure server connections when using SSL",
                Some('k'),
            )
            .switch(
                "allow-errors",
                "do not fail if the server returns an error code",
                Some('e'),
            )
            .param(
                Flag::new("redirect-mode")
                    .short('R')
                    .arg(SyntaxShape::String)
                    .desc(
                        "What to do when encountering redirects. Default: 'follow'. Valid \
                         options: 'follow' ('f'), 'manual' ('m'), 'error' ('e').",
                    )
                    .completion(Completion::new_list(RedirectMode::MODES)),
            )
            .category(Category::Network);
        add_unix_socket_flag(sig)
    }

    fn description(&self) -> &str {
        "Reset builtin http connection pool"
    }

    fn extra_description(&self) -> &str {
        "All connections inside http connection poll will be reset."
    }

    fn search_terms(&self) -> Vec<&str> {
        vec!["network"]
    }

    fn run(
        &self,
        engine_state: &EngineState,
        stack: &mut Stack,
        call: &Call,
        input: PipelineData,
    ) -> Result<PipelineData, ShellError> {
        let args = Arguments {
            insecure: call.has_flag(engine_state, stack, "insecure")?,
            redirect: call.get_flag(engine_state, stack, "redirect-mode")?,
            unix_socket: call.get_flag(engine_state, stack, "unix-socket")?,
        };

        let redirect_mode = http_parse_redirect_mode(args.redirect)?;

        let cwd = engine_state.cwd(None)?;
        let unix_socket_path = expand_unix_socket_path(args.unix_socket, &cwd);
        reset_http_client_pool(
            args.insecure,
            redirect_mode,
            unix_socket_path,
            engine_state,
            stack,
        )?;
        Ok(PipelineData::Empty)
    }
}

struct Arguments {
    insecure: bool,
    redirect: Option<Spanned<String>>,
    unix_socket: Option<Spanned<String>>,
}
