use nu_plugin::{serve_plugin, MsgPackSerializer};
use nu_plugin_polars::PolarsPlugin;

fn main() {
    serve_plugin(&PolarsPlugin::default(), MsgPackSerializer {})
}
