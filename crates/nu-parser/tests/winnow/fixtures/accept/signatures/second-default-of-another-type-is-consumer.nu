# `1..=5` lexes as `1..`, `=`, `5` here; nu parses `5` as a range (the first default's type),
# which it is not.
def f [x = 1..=5] { $x }
