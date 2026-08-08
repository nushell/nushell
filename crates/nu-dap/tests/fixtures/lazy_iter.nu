# Regression: a bare top-level lazy pipeline. `each` returns a lazy stream
# that is the block's return value (no `drain` instruction), so its closure
# only runs when the script's final output is consumed. Breakpoints inside
# the closure (line 2) must still hit.
[10 20 30] | each { |n|
  print $"n=($n)"
}
