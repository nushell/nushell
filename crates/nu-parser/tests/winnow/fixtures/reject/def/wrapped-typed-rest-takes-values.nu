# With a type on the rest parameter the arguments of a wrapped command are values.
def --wrapped f [...rest: string] {}
f 'x'$
