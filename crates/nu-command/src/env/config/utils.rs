use crate::ExternalCommand;
use nu_protocol::{FutureSpanId, OutDest, Spanned};
use std::{collections::HashMap, path::PathBuf};

pub(crate) fn gen_command(
    span: FutureSpanId,
    config_path: PathBuf,
    item: String,
    config_args: Vec<String>,
    env_vars_str: HashMap<String, String>,
) -> ExternalCommand {
    let name = Spanned { item, span };

    let mut args = vec![Spanned {
        item: config_path.to_string_lossy().to_string(),
        span: FutureSpanId::unknown(),
    }];

    let number_of_args = config_args.len() + 1;

    for arg in config_args {
        args.push(Spanned {
            item: arg,
            span: FutureSpanId::unknown(),
        })
    }

    ExternalCommand {
        name,
        args: Spanned {
            item: args,
            span: FutureSpanId::unknown(),
        },
        arg_keep_raw: vec![false; number_of_args],
        out: OutDest::Inherit,
        err: OutDest::Inherit,
        env_vars: env_vars_str,
    }
}
