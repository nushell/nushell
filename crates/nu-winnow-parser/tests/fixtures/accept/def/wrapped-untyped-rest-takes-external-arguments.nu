# nu gives the untyped rest parameter of a `def --wrapped` the `external_arg`
# shape, so its arguments parse as an external command's: `'x'$` and `0b2` are strings.
def --wrapped f [...rest] { ^echo ...$rest }
f -e 'x'$ 0b2
