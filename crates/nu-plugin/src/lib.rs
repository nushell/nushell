pub mod plugin;
pub mod plugin_call;
pub mod serializers;

pub mod plugin_capnp {
    include!(concat!(env!("OUT_DIR"), "/plugin_capnp.rs"));
}
