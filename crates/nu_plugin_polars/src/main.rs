use nu_plugin::{serve_plugin, MsgPackSerializer};
use nu_plugin_polars::PolarsPlugin;

#[global_allocator]
static GLOBAL: mimalloc::MiMalloc = mimalloc::MiMalloc;

fn main() {
    env_logger::init();
    serve_plugin(&PolarsPlugin::default(), MsgPackSerializer {})
}
