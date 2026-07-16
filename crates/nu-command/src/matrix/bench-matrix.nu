use std/bench *

print ""
print "=== MATRIX vs PLAIN NUSHELL (100x100, 100 rounds, release) ==="
print ""

def us [d: duration] { ($d | into int) / 1000.0 | math round | into string }

def ratio [faster: duration, slower: duration] {
    let f = ($faster | into int)
    let s = ($slower | into int)
    if $f == 0 { "inf" } else { ($s / $f | math round -p 1 | into string) }
}

# 1. SUM
let r1 = (bench { matrix zeros 100 100 | matrix add 1 | matrix sum } { 1..100 | each {|_| 1..100 | each {|_| 1.0}} | flatten | math sum } --rounds 100)
let m1m = (us ($r1 | get 0.mean))
let m1n = (us ($r1 | get 1.mean))
let x1 = (ratio ($r1 | get 0.mean) ($r1 | get 1.mean))
print ("1. Sum:       matrix=" + $m1m + "us  nushell=" + $m1n + "us  matrix " + $x1 + "x faster")
print ""

# 2. SCALE
let r2 = (bench { matrix zeros 100 100 | matrix add 1 | matrix scale 2 | matrix into-nu | collect } { 1..100 | each {|_| 1..100 | each {|_| 1.0}} | each {|row| $row | each {|e| $e * 2}} | collect } --rounds 100)
let m2m = (us ($r2 | get 0.mean))
let m2n = (us ($r2 | get 1.mean))
let x2 = (ratio ($r2 | get 0.mean) ($r2 | get 1.mean))
print ("2. Scale:      matrix=" + $m2m + "us  nushell=" + $m2n + "us  matrix " + $x2 + "x faster")
print ""

# 3. TRANSPOSE
let r3 = (bench { matrix zeros 100 100 | matrix add 1 | matrix transpose | matrix into-nu | collect } --rounds 100)
print ("3. Transpose:  " + (us $r3.mean) + "us  --  nushell: NOT POSSIBLE")
print ""

# 4. ADD
let r4 = (bench { matrix zeros 100 100 | matrix add 1 | matrix add (matrix zeros 100 100 | matrix add 2) | matrix into-nu | collect } --rounds 100)
print ("4. Add:        " + (us $r4.mean) + "us  --  nushell: DOES NOT COMPLETE")
print ""

# 5. MULTIPLY
let r5 = (bench { matrix zeros 50 50 | matrix add 1 | matrix multiply (matrix zeros 50 50 | matrix add 1) | matrix into-nu | collect } --rounds 100)
print ("5. Multiply:   " + (us $r5.mean) + "us  --  nushell: O n^3 infeasible")
print ""

# 6. CONSTRUCT
let r6 = (bench { matrix zeros 100 100 | matrix add 1 | matrix into-nu | collect } { 1..100 | each {|_| 1..100 | each {|_| 1.0}} | collect } --rounds 100)
let m6m = (us ($r6 | get 0.mean))
let m6n = (us ($r6 | get 1.mean))
let x6 = (ratio ($r6 | get 0.mean) ($r6 | get 1.mean))
print ("6. Construct:  matrix=" + $m6m + "us  nushell=" + $m6n + "us  matrix " + $x6 + "x faster")
print ""

# 7. AXIS SUM
let r7 = (bench { matrix zeros 100 100 | matrix add 1 | matrix sum --axis 1 | matrix into-nu | collect } { 1..100 | each {|_| 1..100 | each {|_| 1.0}} | each {|row| $row | math sum} | collect } --rounds 100)
let m7m = (us ($r7 | get 0.mean))
let m7n = (us ($r7 | get 1.mean))
let x7 = (ratio ($r7 | get 0.mean) ($r7 | get 1.mean))
print ("7. Axis sum:   matrix=" + $m7m + "us  nushell=" + $m7n + "us  matrix " + $x7 + "x faster")
print ""

# 8. MAP
let r8 = (bench { matrix zeros 100 100 | matrix add 1 | matrix map {|e| $e * 2 + 1} | matrix into-nu | collect } { 1..100 | each {|_| 1..100 | each {|_| 1.0}} | each {|row| $row | each {|e| $e * 2 + 1}} | collect } --rounds 100)
let m8m = (us ($r8 | get 0.mean))
let m8n = (us ($r8 | get 1.mean))
let x8 = (ratio ($r8 | get 0.mean) ($r8 | get 1.mean))
print ("8. Map:        matrix=" + $m8m + "us  nushell=" + $m8n + "us  matrix " + $x8 + "x faster")
print ""

print "=== END ==="
print "  add + multiply: no practical nushell equivalent"
print "  transpose: no nushell equivalent for list-of-lists"
print ""
