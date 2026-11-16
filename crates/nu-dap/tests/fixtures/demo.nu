# Demo script for exercising nu-dap.
# Suggested breakpoints: lines 8, 14, 21.

def classify [size: int] {
    if $size > 1000 { "big" } else { "small" }
}

let files = [
    { name: "a.txt", size: 120 }
    { name: "b.bin", size: 4096 }
    { name: "c.log", size: 900 }
]

mut total = 0
for f in $files {
    let label = (classify $f.size)
    $total = $total + $f.size
    print $"($f.name): ($label)"
}

let summary = { count: ($files | length), total: $total }
print $summary

# Visualizer test data: binary (hex view) and strings (JSON/XML detection).
let blob = 0x[dead beef cafe babe 0011 2233 4455 6677 8899 aabb ccdd eeff]
let payload = '{"user": "ronald", "items": [1, 2, 3], "active": true}'
let markup = '<config><item id="1">alpha</item><item id="2">beta</item></config>'
print ($blob | bytes length)
