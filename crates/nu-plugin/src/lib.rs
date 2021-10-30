pub mod plugin;
pub mod serializers;

pub mod plugin_capnp {
    include!(concat!(env!("OUT_DIR"), "/plugin_capnp.rs"));
}
