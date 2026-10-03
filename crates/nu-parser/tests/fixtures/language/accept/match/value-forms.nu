let x = 1
match $x { _ => 1 }
match ($x + 1) { _ => 1 }
match [1 2] { _ => 1 }
match {a: 1} { _ => 1 }
match "s" { _ => 1 }
match $x.a? { _ => 1 }
match (ls | length) { _ => 1 }
