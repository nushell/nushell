# Multi-file test: breakpoints must work inside sourced files.
source helper.nu
let r = (double 21)
print $r
