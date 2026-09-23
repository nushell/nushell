# Error handling: try/catch, error make, and error propagation.
# Enable "break on exceptions" in the debugger to pause on errors.

# Basic try/catch
print "--- Basic try/catch ---"
try {
    let x = 10 / 0
    print $"result: ($x)"
} catch { |err|
    print $"Caught error: ($err.msg)"
}

# Custom errors with `error make`
def validate_age [age: int] {
    if $age < 0 {
        error make {
            msg: "Invalid age"
            label: {
                text: "age cannot be negative"
                span: (metadata $age).span
            }
        }
    }
    if $age > 150 {
        error make { msg: $"Unrealistic age: ($age)" }
    }
    print $"Age ($age) is valid"
}

print "\n--- Custom errors ---"
try {
    validate_age 25
    validate_age -5
} catch { |err|
    print $"Validation failed: ($err.msg)"
}

# Error propagation — errors bubble up through call stacks
def inner [] {
    error make { msg: "something went wrong deep inside" }
}

def middle [] {
    print "  middle: calling inner..."
    inner
    print "  middle: this never runs"
}

def outer [] {
    print "  outer: calling middle..."
    try {
        middle
    } catch { |err|
        print $"  outer caught: ($err.msg)"
    }
}

print "\n--- Error propagation ---"
outer

# Practical pattern: parse with fallback
def safe_parse_int [s: string] {
    try {
        $s | into int
    } catch {
        null
    }
}

print "\n--- Safe parsing ---"
let inputs = ["42", "hello", "99", "", "7"]
let parsed = $inputs | each { |s|
    let val = safe_parse_int $s
    { input: $s, parsed: $val, ok: ($val != null) }
}
print $parsed
