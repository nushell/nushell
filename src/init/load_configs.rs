use nu_data::config;
use std::path::PathBuf;

use nu_engine::EvaluationContext;
use nu_protocol::ConfigPath;
use nu_source::Text;

pub fn load_cfg_as_global_cfg(context: &EvaluationContext, path: PathBuf) {
    if let Err(err) = context.load_config(&ConfigPath::Global(path)) {
        context
            .host
            .lock()
            .print_err(err, &Text::from("".to_string()));
    }
}

pub fn load_global_cfg(context: &EvaluationContext) {
    match config::default_path() {
        Ok(path) => {
            load_cfg_as_global_cfg(context, path);
        }
        Err(e) => {
            context.host.lock().print_err(e, &Text::from(""));
        }
    }
}
