use nu_plugin::{serve_plugin, MsgPackSerializer};
use nu_plugin_http::HttpPlugin;

#[global_allocator]
static GLOBAL: mimalloc::MiMalloc = mimalloc::MiMalloc;

fn main() {
    serve_plugin(&HttpPlugin::default(), MsgPackSerializer {})
}
