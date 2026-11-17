# Control flow: if/else, match, for, while, loop.
# Good for practicing step-over vs step-into and watching branch selection.

let score = 85

# if / else if / else — step through to see which branch is taken.
let grade = if $score >= 90 {
    "A"
} else if $score >= 80 {
    "B"
} else if $score >= 70 {
    "C"
} else {
    "F"
}
print $"Score ($score) → grade ($grade)"

# match expression
let status_code = 404
let message = match $status_code {
    200 => "OK"
    301 => "Moved"
    404 => "Not Found"
    500 => "Server Error"
    _ => "Unknown"
}
print $"HTTP ($status_code): ($message)"

# for loop — watch `i` update each iteration in the Variables panel.
mut sum = 0
for i in 1..5 {
    $sum = $sum + $i
    print $"  i=($i) sum=($sum)"
}
print $"Total: ($sum)"

# while loop
mut n = 10
while $n > 0 {
    $n = $n - 3
    print $"  n=($n)"
}

# loop with break
mut attempts = 0
loop {
    $attempts = $attempts + 1
    if $attempts >= 3 {
        break
    }
}
print $"Broke after ($attempts) attempts"
