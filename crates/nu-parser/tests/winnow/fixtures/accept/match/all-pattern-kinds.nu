let x = 1
match $x {
  1 | 2 => "low",
  3..5 => { print mid; "mid" }
  $n if $n > 100 => "huge"
  [$a, $b, ..$rest] => $a
  [..$last] => $last
  [..] => "rest"
  {name: $name, age: 3} => $name
  {$shorthand} => 1
  "str" => 2
  (1 + 1) => 3
  {a: 1} => {b: 2}
  _ => {|| 1}
  null => print
}
