fn main() {
    capnpc::CompilerCommand::new()
        .src_prefix("schema")
        .file("schema/plugin.capnp")
        .run()
        .expect("compiling schema");
}
