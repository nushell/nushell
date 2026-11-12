match 1 {
  1 => 1
  2 => "s"
  3 => { 1 }
  4 => { let x = 1; $x }
  5 => {|| 1}
  6 => {a: 1}
  7 => [1 2]
  8 => (1 + 1)
  9 => $env.PWD
  10 => (if true { 1 } else { 2 })
  11 => print
  12 => (ls | length)
  13 => { print a; print b }
  14 => ^ls
  15 => $"x(1)"
  _ => null
}
