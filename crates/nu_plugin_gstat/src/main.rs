use nu_plugin::{MsgPackSerializer, serve_plugin};
use nu_plugin_gstat::GStatPlugin;

fn main() {
    serve_plugin(&GStatPlugin, MsgPackSerializer {})
}
