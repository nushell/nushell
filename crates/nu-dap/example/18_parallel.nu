# Parallel execution: `par-each` for concurrent work.
# Note: breakpoints inside `par-each` closures may behave differently
# than sequential `each` because work runs on multiple threads.

let urls = [
    "https://jsonplaceholder.typicode.com/todos/1"
    "https://jsonplaceholder.typicode.com/todos/2"
    "https://jsonplaceholder.typicode.com/todos/3"
    "https://jsonplaceholder.typicode.com/todos/4"
    "https://jsonplaceholder.typicode.com/todos/5"
]

# Sequential fetch (for comparison / debugging — steppable)
print "--- Sequential fetch ---"
let seq_results = $urls | each { |url|
    let resp = http get $url
    { id: $resp.id, title: $resp.title }
}
print ($seq_results | first 3)

# Parallel fetch — faster but less debuggable
print "\n--- Parallel fetch ---"
let par_results = $urls | par-each { |url|
    let resp = http get $url
    { id: $resp.id, title: $resp.title }
}
print $par_results

# CPU-bound parallel work
print "\n--- Parallel computation ---"
let inputs = 1..10 | each { $in }
let computed = $inputs | par-each { |n|
    # Simulate work
    let result = 1..$n | reduce --fold 1 { |it, acc| $acc * $it }
    { n: $n, factorial: $result }
}
print ($computed | sort-by n)
