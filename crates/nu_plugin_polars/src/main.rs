use nu_plugin::{serve_plugin, MsgPackSerializer};
use nu_plugin_polars::PolarsPlugin;

fn main() {
    env_logger::init();

    match PolarsPlugin::new() {
        Ok(ref plugin) => serve_plugin(plugin, MsgPackSerializer {}),
        Err(e) => {
            eprintln!("{}", e);
            std::process::exit(1);
        }
    }
}
