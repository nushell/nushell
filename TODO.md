# Todo
- [x] Env shorthand
- [x] String interpolation
- [x] Aliases
- [x] Env vars
- [x] Sub commands
- [x] Floats
- [x] Tests
- [x] Decl requires $ but shouldn't
- [x] alias highlighting at call site
- [x] refactor into subcrates
- [x] subcommand alias
- [x] type inference from successful parse (eg not `List<unknown>` but `List<int>`)
- [x] parsing tables
- [x] Block params
- [x] Ranges
- [x] Column path
- [x] ...rest without calling it rest
- [x] Iteration (`each`) over tables
- [x] Row conditions
- [x] Simple completions
- [x] Detecting `$it` currently only looks at top scope but should find any free `$it` in the expression (including subexprs)
- [x] Signature needs to make parameters visible in scope before block is parsed
- [x] Externals
- [x] Modules and imports
- [x] Exports
- [x] Source
- [x] Error shortcircuit (stopping on first error). Revised: errors emit first, but can be seen by commands.
- [x] Value serialization
- [x] Handling rows with missing columns during a cell path
- [x] finish operator type-checking
- [x] Config file loading
- [x] block variable captures
- [x] improved history and config paths
- [x] ctrl-c support
- [x] operator overflow
- [x] Support for `$in`
- [x] config system
- [x] plugins
- [ ] external plugin signatures
- [ ] external command signatures
- [ ] shells
- [ ] autoenv
- [ ] dataframes
- [ ] overlays (replacement for `autoenv`), adding modules to shells
- [ ] port over `which` logic
- [ ] port test support crate so we can test against sample files, including multiple inputs into the CLI
- [ ] benchmarking
- [ ] finish adding config properties
- [ ] system-agnostic test cases
- [ ] exit codes
- [ ] length of time the command runs put in the env (CMD_DURATION_MS)

## Post-nushell merge:
- [ ] Input/output types
- [ ] let [first, rest] = [1, 2, 3] (design question: how do you pattern match a table?)

## Maybe: 
- [ ] default param values?
- [ ] Unary not?



module git {
    external fn "git clone" [
        arg: int,
        --flag: string(custom-completion),   ????
    ] ;
}

plugin git { ... }