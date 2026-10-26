def f [--flag, --val: any, -f, ...rest] { }
let v = {a: 1}
f 1 1.5 true null "s" 's' `b` r#'r'# bare $v $v.a? (1 + 1) [1] {a: 1} {|| 1} 1kb 1sec 2024-01-02 0x[ff] 1..2 $.a $"i(1)" ...[1] --flag -f --val=1 -- -x
