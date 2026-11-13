# Data structures: records, tables, nested data, and binary.
# Expand variables in the debugger's Variables panel to explore deeply
# nested structures. The data visualizer shows tables as grids.

# Simple record
let server = {
    host: "api.example.com"
    port: 443
    protocol: "https"
    timeout_ms: 5000
}
print $"Connecting to ($server.host):($server.port)"

# Table (list of records with consistent columns)
let inventory = [
    { item: "Widget A", qty: 150, price: 9.99, category: "hardware" }
    { item: "Gadget B", qty: 42, price: 24.50, category: "electronics" }
    { item: "Doohickey", qty: 300, price: 2.99, category: "hardware" }
    { item: "Thingamajig", qty: 8, price: 199.00, category: "electronics" }
]
print $inventory

# Deeply nested data — the Variables panel hydrates lazily
let org = {
    name: "Acme Corp"
    departments: [
        {
            name: "Engineering"
            teams: [
                { name: "Backend", members: ["Alice", "Bob", "Carol"] }
                { name: "Frontend", members: ["Dave", "Eve"] }
            ]
        }
        {
            name: "Marketing"
            teams: [
                { name: "Content", members: ["Frank", "Grace"] }
            ]
        }
    ]
    metadata: {
        founded: 1990
        locations: ["NYC", "SF", "London"]
        active: true
    }
}
print $"Company: ($org.name)"
print $"First team: ($org.departments.0.teams.0.name)"

# Binary data — displays as hex in the data visualizer
let magic_bytes = 0x[89 50 4e 47 0d 0a 1a 0a]  # PNG header
let packet = 0x[45 00 00 3c 1c 46 40 00 40 06 b1 e6 ac 10 0a 63 ac 10 0a 0c]
print $"PNG header: ($magic_bytes | bytes length) bytes"
print $"IP packet: ($packet | bytes length) bytes"

# Record manipulation
let base_config = { debug: false, log_level: "info", port: 3000 }
let dev_overrides = { debug: true, log_level: "trace" }
let dev_config = $base_config | merge $dev_overrides
print $dev_config

# Table operations
let expensive = $inventory | where { $in.price > 10 } | sort-by price --reverse
let by_category = $inventory | group-by category
print $"Expensive items: ($expensive | get item)"
print $"Categories: ($by_category | columns)"
