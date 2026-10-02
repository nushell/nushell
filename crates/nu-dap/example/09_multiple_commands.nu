# Multiple commands: several `def` blocks that can each be chosen as the
# entry point. The debugger shows a picker when there's no `main` and no
# top-level code beyond definitions.

def hello [name: string = "world"] {
    print $"Hello, ($name)!"
}

def countdown [from: int = 5] {
    mut n = $from
    while $n > 0 {
        print $"  ($n)..."
        $n = $n - 1
    }
    print "  Go!"
}

def fizzbuzz [limit: int = 20] {
    for i in 1..$limit {
        let result = if $i mod 15 == 0 {
            "FizzBuzz"
        } else if $i mod 3 == 0 {
            "Fizz"
        } else if $i mod 5 == 0 {
            "Buzz"
        } else {
            $"($i)"
        }
        print $"  ($result)"
    }
}

def stats [numbers: list<int>] {
    let sum = $numbers | math sum
    let avg = $numbers | math avg
    let min = $numbers | math min
    let max = $numbers | math max
    print { sum: $sum, avg: $avg, min: $min, max: $max }
}
