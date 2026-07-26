# File operations: listing, reading, and path manipulation.
# These work on the actual filesystem — watch the results in Variables.

# List files in the current directory
let entries = ls | select name type size
print $"Files in current dir:"
print ($entries | first 5)

# Path operations
let filepath = "/home/user/documents/report.pdf"
let parsed = $filepath | path parse
print $"\nPath parts:"
print $"  parent: ($parsed.parent)"
print $"  stem: ($parsed.stem)"
print $"  extension: ($parsed.extension)"

# Working with globs
let nu_files = ls *.nu | get name
print $"\n.nu files here: ($nu_files)"

# File size analysis
let all_files = ls *.nu | select name size | sort-by size --reverse
print $"\nFiles by size:"
print $all_files

# Path construction
let parts = { parent: "/tmp", stem: "output", extension: "json" }
let built_path = [$parts.parent, $"($parts.stem).($parts.extension)"] | path join
print $"\nBuilt path: ($built_path)"

# Check existence
let targets = ["./helper.nu", "./nonexistent.nu", "./demo.nu"]
let existence = $targets | each { |t|
    { path: $t, exists: ($t | path exists) }
}
print $"\nExistence check:"
print $existence
