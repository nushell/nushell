use module.nu "module name"
overlay use module.nu as new_name
overlay hide --keep-env [PWD] new_name
hide "module name"

# stdlib files are loaded virtually
# goto_def on them should just fail without panic
use std/log log-level
