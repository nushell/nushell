use nu_data::config;
use std::path::PathBuf;

use nu_engine::EvaluationContext;
use nu_protocol::{ConfigPath, UntaggedValue};
use nu_source::Text;

pub async fn load_cfg_as_global_cfg(context: &EvaluationContext, path: PathBuf) {
    if let Err(err) = context.load_config(&ConfigPath::Global(path.clone())).await {
        context.host.lock().print_err(err, &Text::empty());
    } else {
        //TODO current commands assume to find path to global cfg file under config-path
        //TODO use newly introduced nuconfig::file_path instead
        context.scope.add_var(
            "config-path",
            UntaggedValue::filepath(path).into_untagged_value(),
        );
    }
}

pub async fn load_global_cfg(context: &EvaluationContext) {
    match config::default_path() {
        Ok(path) => {
            load_cfg_as_global_cfg(context, path).await;
        }
        Err(e) => {
            context.host.lock().print_err(e, &Text::from(""));
        }
    }
}
