@example "hello world" { 1 }
@example "int" { 42 } --result 42
@example "null" { null } --result null
@example "list" { [1 2] } --result [1 2]
@example "record" { {a: 1} } --result {a: 1}
@example "math" { (1 + 1) } --result 2
@example "interp" { $"x(1)" } --result "x1"
@example "mixed" { [true 1.5 1kb] }
@search-terms a b c
@category math
def foo [] {}
