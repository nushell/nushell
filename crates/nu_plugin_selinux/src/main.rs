use nu_plugin::{serve_plugin, MsgPackSerializer};
use nu_plugin_selinux::SELinuxPlugin;

fn main() {
    serve_plugin(&SELinuxPlugin {}, MsgPackSerializer {})
}
