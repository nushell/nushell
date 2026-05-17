use nu_plugin::{MsgPackSerializer, serve_plugin};
use nu_plugin_formats::FormatCmdsPlugin;

fn main() {
    serve_plugin(&FormatCmdsPlugin, MsgPackSerializer {})
}
