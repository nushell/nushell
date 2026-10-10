let x = {a: 1}
match $x {
  {a: # the key
   $y} => $y
  _ => null
}
