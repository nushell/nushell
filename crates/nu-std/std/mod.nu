# std.nu, used to load all standard library components

# Top-level commands: ellie, repeat, null-device, and "path add"
export use std/util *

# std submodules
export use std/assert
export use std/bench
export use std/dt
export use std/formats
export use std/help
export use std/input
export use std/iter
export use std/log
export use std/math
export use std/xml
export use std/config
export use std/testing
export use std/random
export use std/dirs

# Workaround for #13403 to load export-env blocks from submodules
export-env {
    use std/log []
    use std/dirs []
}
