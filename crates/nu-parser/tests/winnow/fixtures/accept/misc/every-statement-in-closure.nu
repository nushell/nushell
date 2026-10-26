do {
  let a = 1
  mut b = 2
  const c = 3
  def f [] { }
  alias g = f
  module m { }
  use m
  for x in [1] { }
  while false { }
  loop { break }
  if true { } else { }
  match 1 { _ => 1 }
  try { } catch { }
  $b = 4
  ^echo x
  FOO=1 echo y
  return 1
}
