let y = 1
match 1 {
  $x if $x > 0 => pos
  $x if ($x mod 2) == 0 => even
  $x if $x in [1 2] and $y == 1 => complex
  _ if true => fallback
}
