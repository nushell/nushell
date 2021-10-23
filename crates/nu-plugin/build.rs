fn main() {
    capnpc::CompilerCommand::new()
        .src_prefix("schema")
        .file("schema/value.capnp")
        .run()
        .expect("compiling schema");
}
