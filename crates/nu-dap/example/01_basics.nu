# Basics: variables, types, and simple expressions.
# Try setting a breakpoint on each `let` and stepping through to watch
# the Variables panel populate with different value types.

let name = "nushell"
let version = 114
let pi = 3.14159
let active = true
let nothing = null
let size = 2kb

# Mutable variables — watch the value change as you step.
mut counter = 0
$counter = $counter + 1
$counter = $counter + 5
$counter = $counter * 2

# Collections
let colors = ["red", "green", "blue"]
let config = { host: "localhost", port: 8080, debug: true }
let matrix = [[1, 2, 3], [4, 5, 6], [7, 8, 9]]

# Binary data (displays in hex view in the visualizer)
let blob = 0x[de ad be ef ca fe ba be]

# String interpolation
let greeting = $"Hello ($name) v($version)!"
print $greeting

# Type checking
print ($name | describe)
print ($config | describe)
print ($colors | describe)
