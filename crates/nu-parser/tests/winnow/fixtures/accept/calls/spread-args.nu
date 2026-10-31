def foo [...rest] { $rest }
let args = [1 2]
foo ...$args ...[3 4] ...(5..6 | each { $in }) 7
