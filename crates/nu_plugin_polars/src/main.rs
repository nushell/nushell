use nu_plugin::{serve_plugin, MsgPackSerializer};
use nu_plugin_polars::PolarsDataFramePlugin;

fn main() {
    serve_plugin(&PolarsDataFramePlugin {}, MsgPackSerializer {})
}
