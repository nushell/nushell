use nu_plugin::{serve_plugin, MsgPackSerializer};
use nu_plugin_gstat::GStatPlugin;

fn main() {
    serve_plugin(&GStatPlugin, MsgPackSerializer {})
}
