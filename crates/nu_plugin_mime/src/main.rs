use nu_plugin::{serve_plugin, MsgPackSerializer};
use nu_plugin_mime::Mime;

fn main() {
    serve_plugin(&Mime {}, MsgPackSerializer {})
}
