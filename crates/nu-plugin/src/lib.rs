pub mod plugin;
pub mod serializers;

pub mod value_capnp {
    include!(concat!(env!("OUT_DIR"), "/value_capnp.rs"));
}
