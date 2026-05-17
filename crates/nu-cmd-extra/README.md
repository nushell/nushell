# nu-cmd-extra

The commands in this crate are the *extra commands* of Nushell.  These commands
are not in a state to be guaranteed to be part of the 1.0 API; meaning that
there is no guarantee longer term that these commands will be around into the
future.

For a while we did exclude them behind the `--features extra` compile time
flag, meaning that the default release did not contain them. As we (the Nushell
team) shipped a full build including both `extra` and `dataframe` for some
time, we chose to sunset the `extra` feature but keep the commands in this
crate for now. In the future the commands may be moved to more topical crates
or discarded into plugins.
