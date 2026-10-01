def foo []: [int -> int, string -> int] {
    let x = $in
    if ($x | describe) == "int" { 3 } else { 4 }
}
