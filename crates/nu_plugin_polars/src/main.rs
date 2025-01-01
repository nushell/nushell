use nu_plugin::{serve_plugin, MsgPackSerializer};
use nu_plugin_polars::PolarsPlugin;

#[global_allocator]
static GLOBAL: mimalloc::MiMalloc = mimalloc::MiMalloc;

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
