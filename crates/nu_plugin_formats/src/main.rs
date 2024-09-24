use nu_plugin::{serve_plugin, MsgPackSerializer};
use nu_plugin_formats::FormatCmdsPlugin;

fn main() {
    serve_plugin(&FormatCmdsPlugin, MsgPackSerializer {})
}
