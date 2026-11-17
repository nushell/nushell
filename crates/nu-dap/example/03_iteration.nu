# Iteration: each, where, reduce, and closure parameters.
# Demonstrates the `|it|` closure variable and various iteration patterns.

let numbers = [1, 2, 3, 4, 5, 6, 7, 8, 9, 10]

# `each` with explicit closure parameter — step into the closure to
# watch `it` take a new value on every iteration.
let doubled = $numbers | each { |it|
    $it * 2
}
print $"Doubled: ($doubled)"

# `where` with closure parameter — filtering
let evens = $numbers | where { |it| $it mod 2 == 0 }
print $"Evens: ($evens)"

# `each` with named parameter for clarity
let squares = $numbers | each { |n|
    $n * $n
}
print $"Squares: ($squares)"

# Working with tables — `each` over rows
let people = [
    { name: "Alice", age: 30 }
    { name: "Bob", age: 25 }
    { name: "Carol", age: 35 }
]

let introductions = $people | each { |person|
    $"($person.name) is ($person.age) years old"
}
print $introductions

# `reduce` — accumulating a value (watch `acc` grow)
let total = $numbers | reduce { |it, acc|
    $acc + $it
}
print $"Sum via reduce: ($total)"

# `reduce` with initial value
let product = [2, 3, 4] | reduce --fold 1 { |it, acc|
    $acc * $it
}
print $"Product: ($product)"

# Nested iteration
let grid = [[1, 2], [3, 4], [5, 6]]
let flat = $grid | each { |row|
    $row | each { |cell|
        $cell * 10
    }
} | flatten
print $"Flattened grid: ($flat)"

# `enumerate` — get index alongside value
let indexed = ["a", "b", "c"] | enumerate | each { |it|
    $"($it.index): ($it.item)"
}
print $indexed
