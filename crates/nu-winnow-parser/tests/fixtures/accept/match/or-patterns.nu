match 1 {
  1 | 2 | 3 => low
  "a" | "b" => str
  [$a] | [$a $b] => list
  _ | _ => other
}
