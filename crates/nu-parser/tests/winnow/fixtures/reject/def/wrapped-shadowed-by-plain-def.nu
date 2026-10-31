# A plain `def` of the same name in an inner block shadows the wrapped one.
def --wrapped f [...rest] {}
do { def f [...rest] {}; f 'x'$ }
