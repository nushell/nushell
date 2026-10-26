# nu parses a second default of a parameter without a type with the type of the first.
def f [x = a = 1, --n = 1 = 2] { $x }
