# A lone `let` reads the redirection's items as its own arguments, which fails.
let x --help o> f
