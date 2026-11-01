# Step-into regression: the closure body lives on the SAME line as the
# pipeline, so F11 must key on depth change, not just line change.
let nums = [1 2 3]
let doubled = ($nums | each {|n| $n * 2 })
print $doubled
let tripled = ($nums | each { $in * 3 })
print $tripled
$env.NU_DAP_TEST = "hello-env"
print "env set"
let s = ("a-b" | split row "-" | get 0 | str upcase)
print $s
