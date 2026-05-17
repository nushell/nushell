# nu_plugin_formats
A nushell plugin to convert data to nushell tables.

# support commands:
1. from eml - original ported from nushell core.
2. from ics - original ported from nushell core.
3. from ini - original ported from nushell core.
4. from vcf - original ported from nushell core.
5. from plist - original ported from nushell core.
6. to plist - original ported from nushell core.

# Prerequisite
`nushell`, It's a nushell plugin, so you need it.

# Usage
1. compile the binary: `cargo build`
2. plugin add plugin(assume it's compiled in ./target/debug/):
```
plugin add ./target/debug/nu_plugin_formats
```
