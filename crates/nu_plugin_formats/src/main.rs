use nu_plugin::{serve_plugin, MsgPackSerializer};
use nu_plugin_formats::FromCmds;

fn main() {
    serve_plugin(&FromCmds, MsgPackSerializer {})
}
