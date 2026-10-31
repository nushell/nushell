# After the `--` that ends the options, `--help` is a member, not a call for help
# (nu has no flag after it); whether std exports such a name is the consumer's.
use std -- --help
