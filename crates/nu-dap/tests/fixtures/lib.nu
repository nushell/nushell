# A function library: no `main`, no top-level code. The debugger lets you
# pick an entry point (e.g. `greet`) to run and step through.
def greet [name: string] {
    let msg = $"hello ($name)"
    print $msg
    $msg
}

def add [a: int, b: int] {
    $a + $b
}
