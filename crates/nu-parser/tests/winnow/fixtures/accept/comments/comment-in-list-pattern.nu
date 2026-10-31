let x = [1 2]
match $x {
  [1 # the first
   2] => "pair"
  _ => "other"
}
