def f [] {
  ls | upsert a {|n|
    $n | length
  | upsert b {|x|
    $x
  }
