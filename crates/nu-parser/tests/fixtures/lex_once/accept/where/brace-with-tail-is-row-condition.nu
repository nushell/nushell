# nu tries a closure first and falls back to a row condition, in which `{}.a`
# is a cell path on an empty record.
[0 1 2] | where {}.a
