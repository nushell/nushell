# nu-cmd-extra

## the extra commands are not part of the Nushell binary

The commands in this crate are the *extra commands* of Nushell.  They do not
get built for the release and it is the responsibility of the developer to
build these commands if they want to use them.

These commands are not going to part of the 1.0 Api; meaning that there
is no guarantee longer term that these commands will be around into the future.
Of course since they are part of the source tree one could always incorporate
them into their own custom release.

### How to build the commands in this crate

Step 1 is to
[read the installation notes](https://www.nushell.sh/book/installation.html#build-from-source)
for Nushell which is located in our Nushell book.

Once Rust is installed you can then build Nushell with the following command.

```rust
cargo build --features=extra
```

Your Nushell binary which just got built is called *nu* and will be located here.

```
nushell/target/debug/nu
```
