use nu_plugin::{serve_plugin, MsgPackSerializer};
use nu_plugin_gstat::GStat;

fn main() {
    serve_plugin(&GStat::new(), MsgPackSerializer {})
}
