let deep = [
  {a: [1 {b: $"x([1 2] | each {|i| {c: (1 + $i)} } | length)"}]}
  (ls | where size > 1kb | each { |f| {name: $f.name, n: ($f.name | str length)} })
]
$deep.0.a.1.b
