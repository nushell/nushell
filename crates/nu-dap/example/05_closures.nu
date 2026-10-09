# Closures: capturing variables, passing closures, higher-order patterns.
# Step into closures to see captured values in the Variables panel.

# A closure captures variables from its defining scope
let multiplier = 3
let multiply = { |x| $x * $multiplier }
let result = do $multiply 7
print $"3 × 7 = ($result)"

# Closures as arguments — the classic functional pattern
def apply_twice [f: closure, value: int] {
    let once = do $f $value
    let twice = do $f $once
    $twice
}

let doubled_twice = apply_twice { |x| $x * 2 } 5
print $"5 → double → double = ($doubled_twice)"

# Closure with enumerate — numbering items in a list
let items = ["apple", "banana", "cherry"]
$items | enumerate | each { |entry|
    $"($entry.index + 1). ($entry.item)"
} | print

# Building a pipeline transform with closures
def transform [data: list, steps: list] {
    mut current = $data
    for step in $steps {
        $current = $current | each { |it| do $step $it }
    }
    $current
}

let pipeline_result = transform [1, 2, 3, 4] [
    { |x| $x * 10 }
    { |x| $x + 1 }
]
print $"Transform result: ($pipeline_result)"

# Closure in a record (strategy pattern)
let formatter = {
    upper: { |s| $s | str upcase }
    lower: { |s| $s | str downcase }
    title: { |s| $s | str capitalize }
}

let text = "hello WORLD"
print (do $formatter.upper $text)
print (do $formatter.lower $text)
print (do $formatter.title $text)
