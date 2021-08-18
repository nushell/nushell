# how to compile without OpenSSL

You may find it desirable to compile nu shell without requiring an OpenSSL installation on your system.

You can do this by runnning:
```sh
cargo build --no-default-features --features=rustyline-support
```

