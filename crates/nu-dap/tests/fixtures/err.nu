# Exception-breakpoint test: raises a runtime error on line 4.
let a = 1
let b = 2
error make {msg: "boom"}
print "unreachable"
