# Definitions + top-level code: functions are defined, then called at the
# top level. This is the most common script pattern. Breakpoints work in
# both the function bodies and the top-level calls.

def classify [size: int] {
    if $size > 1000 {
        "large"
    } else if $size > 100 {
        "medium"
    } else {
        "small"
    }
}

def summarize [items: list] {
    let total = $items | get size | math sum
    let count = $items | length
    { count: $count, total_size: $total, average: ($total / $count) }
}

# --- Top-level execution starts here ---

let files = [
    { name: "app.js", size: 4200 }
    { name: "style.css", size: 850 }
    { name: "icon.png", size: 50 }
    { name: "data.json", size: 12000 }
]

# Call our functions — step into to enter the function body
mut results = []
for f in $files {
    let label = classify $f.size
    $results = ($results | append $"($f.name): ($label)")
    print $"  ($f.name) → ($label)"
}

let stats = summarize $files
print $"Summary: ($stats)"
print $"Results: ($results)"
