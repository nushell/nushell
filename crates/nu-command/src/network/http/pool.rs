use crate::network::http::client::{
    RedirectMode, add_unix_socket_flag, expand_unix_socket_path, http_parse_redirect_mode,
    reset_http_client_pool,
};
use nu_engine::command_prelude::*;

#[derive(Clone)]
pub struct HttpPool;

impl Command for HttpPool {
    fn name(&self) -> &str {
        "http pool"
    }

    fn signature(&self) -> Signature {
        let sig = Signature::build("http pool")
            .input_output_types(vec![(Type::Nothing, Type::Any)])
            .allow_variants_without_examples(true)
            .switch(
                "insecure",
                "Allow insecure server connections when using SSL.",
                Some('k'),
            )
            .switch(
                "allow-errors",
                "Do not fail if the server returns an error code.",
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
        "Configure and reset builtin http connection pool."
    }

    fn extra_description(&self) -> &str {
        "All connections inside http connection pool will be closed."
    }

    fn search_terms(&self) -> Vec<&str> {
        vec!["network"]
    }

    fn run(
        &self,
        engine_state: &EngineState,
        stack: &mut Stack,
        call: &Call,
        _input: PipelineData,
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
