use std::collections::HashMap;
use std::path::PathBuf;

use nu_protocol::{Span, Spanned};

use crate::ExternalCommand;

pub(crate) fn gen_command(
    span: Span,
    config_path: PathBuf,
    item: String,
    config_args: Vec<String>,
    env_vars_str: HashMap<String, String>,
) -> ExternalCommand {
    let name = Spanned { item, span };

    let mut args = vec![Spanned {
        item: config_path.to_string_lossy().to_string(),
        span: Span::unknown(),
    }];

    let number_of_args = config_args.len() + 1;

    for arg in config_args {
        args.push(Spanned {
            item: arg,
            span: Span::unknown(),
        })
    }

    ExternalCommand {
        name,
        args,
        arg_keep_raw: vec![false; number_of_args],
        redirect_stdout: false,
        redirect_stderr: false,
        redirect_combine: false,
        env_vars: env_vars_str,
        trim_end_newline: false,
    }
}
