use nu_plugin::{serve_plugin, EncodingType};
use nu_plugin_formats::FromCmds;

fn main() {
    serve_plugin(&mut FromCmds, EncodingType::MsgPack)
}
