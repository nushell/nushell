# Deep nesting (lazy hydration) + a long multibyte string (UTF-8 truncation).
let deep = {l1: {l2: {l3: {l4: {l5: {l6: "bottom"}}}}}}
let uni = "éééééééééééééééééééééééééééééééééééééééééééééééééééééééééééééééééééééééééééééééééééééééééééééééééééééééééééééééééééééééééééééééééé"
print "deep done"
