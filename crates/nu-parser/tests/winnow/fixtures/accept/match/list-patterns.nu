match [1 2 3] {
  [] => empty
  [$a] => one
  [$a $b] => two
  [$a, $b, $c] => three
  [1 $b _] => lit
  [$head ..$tail] => rest
  [..$last] => last_only
  [..] => anything
  [[$x] [$y]] => nested
  [{a: $v}] => record_in_list
}
