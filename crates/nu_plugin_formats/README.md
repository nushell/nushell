# nu_plugin_formats

A nushell plugin to convert data to nushell tables.

# support commands:

1. from eml - original ported from nushell core.
1. from ics - original ported from nushell core.
1. from ini - original ported from nushell core.
1. from vcf - original ported from nushell core.

# Prerequisite

`nushell`, It's a nushell plugin, so you need it.

# Usage

1. compile the binary: `cargo build`
1. register plugin(assume it's compiled in ./target/debug/):

```
register ./target/debug/nu_plugin_formats
```
