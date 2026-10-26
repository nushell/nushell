def c [] { c }
def px [n=0] { let l = $in; if $n == 0 { return false } else { $l | px ($n - 1) } }
