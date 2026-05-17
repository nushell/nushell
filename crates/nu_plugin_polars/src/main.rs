use nu_plugin::{MsgPackSerializer, serve_plugin};
use nu_plugin_polars::PolarsPlugin;

fn main() {
    env_logger::init();

    // Set config options via environment variable
    unsafe {
        // Extensions are required for certain things like aggregates with object dtypes to work
        // correctly. It is disabled by default because of unsafe code.
        // See https://docs.rs/polars/latest/polars/#user-guide for details
        std::env::set_var("POLARS_ALLOW_EXTENSION", "true");
    }
    match PolarsPlugin::new() {
        Ok(ref plugin) => serve_plugin(plugin, MsgPackSerializer {}),
        Err(e) => {
            eprintln!("{e}");
            std::process::exit(1);
        }
    }
}
