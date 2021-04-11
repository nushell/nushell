use std::path::PathBuf;

use nu_engine::EvaluationContext;
use nu_protocol::{UntaggedValue, Value};
use nu_source::{Tag, Text};

pub fn load_plugins(context: &EvaluationContext) {
    match nu_engine::plugin::build_plugin::scan(search_paths()) {
        Ok(plugins) => {
            context.add_commands(
                plugins
                    .into_iter()
                    .filter(|p| !context.is_command_registered(p.name()))
                    .collect(),
            );
        }
        Err(e) => {
            context
                .host
                .lock()
                .print_err(e, &Text::from("".to_string()));
        }
    }
}

fn search_paths() -> Vec<std::path::PathBuf> {
    use std::env;

    let mut search_paths = Vec::new();

    // Automatically add path `nu` is in as a search path
    if let Ok(exe_path) = env::current_exe() {
        if let Some(exe_dir) = exe_path.parent() {
            search_paths.push(exe_dir.to_path_buf());
        }
    }

    if let Ok(config) = nu_data::config::config(Tag::unknown()) {
        if let Some(Value {
            value: UntaggedValue::Table(pipelines),
            ..
        }) = config.get("plugin_dirs")
        {
            for pipeline in pipelines {
                if let Ok(plugin_dir) = pipeline.as_string() {
                    search_paths.push(PathBuf::from(plugin_dir));
                }
            }
        }
    }
    search_paths
}
