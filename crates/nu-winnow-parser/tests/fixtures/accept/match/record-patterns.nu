match {a: 1, b: {c: 2}} {
  {} => empty
  {a: 1} => lit
  {a: $a} => bind
  {a: $a, b: $b} => two
  {a: $a b: {c: $c}} => nested
  {$a} => shorthand
  {$a $b} => shorthands
  {"quoted key": $q} => quoted
  {a: [$x ..$rest]} => list_in_record
}
