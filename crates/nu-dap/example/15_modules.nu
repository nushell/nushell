# Modules and sourcing: importing code from other files.
# Breakpoints work inside sourced files — try setting one in helper.nu.

# `source` loads definitions into the current scope
source helper.nu

# Use the imported `double` function
let x = double 21
print $"double 21 = ($x)"

let y = double 100
print $"double 100 = ($y)"

# Multiple calls — step into each to enter helper.nu
let values = [3, 7, 15, 22]
let doubled_list = $values | each { |v| double $v }
print $"Doubled list: ($doubled_list)"
