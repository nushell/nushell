def f [--x: int, ...rest] { [$x $rest] }
f ...{x: 1} ...{rest: [a b]}
