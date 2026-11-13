# Definitions only — a pure library file with no top-level executable code.
# The debugger lets you pick an entry point to run (e.g. `greet "world"`).
# This exercises the "no main, no top-level" launch path.

# Simple function
def greet [name: string] {
    let msg = $"Hello, ($name)!"
    print $msg
    $msg
}

# Function with multiple params and a default
def repeat_string [text: string, count: int = 3] {
    let parts = 1..$count | each { $text }
    $parts | str join " "
}

# Function that returns a table
def make_report [items: list] {
    $items | each { |item|
        {
            name: $item
            length: ($item | str length)
            upper: ($item | str upcase)
        }
    }
}

# Recursive function (fibonacci)
def fib [n: int] {
    if $n <= 1 {
        $n
    } else {
        (fib ($n - 1)) + (fib ($n - 2))
    }
}

# Function with flag parameters
def search [pattern: string, --case-sensitive, --max-results: int = 10] {
    let mode = if $case_sensitive { "exact" } else { "fuzzy" }
    print $"Searching '($pattern)' mode=($mode) max=($max_results)"
}
