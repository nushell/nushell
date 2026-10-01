# `$x.c!!` does not parse as a cell path, so `..$x.c!!` is no range: an external command.
let x = {a: 1}
..$x.c!!
