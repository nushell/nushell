# External processes: spawning system commands and capturing output.
# Watch how external output appears in the Debug Console.

# Run an external command — its output goes through the DAP output channel
print "--- System info ---"
^hostname

# Capture external output into a variable
let py_version = ^python --version
print $"Python: ($py_version)"

# External with arguments — useful for build scripts
let file_count = ^git ls-files | lines | length
print $"Files in repo: ($file_count)"

# Piping nushell data to an external and back
let json_data = { name: "test", value: 42 } | to json
let pretty = $json_data | ^python -c "import sys,json; print(json.dumps(json.loads(sys.stdin.read()), indent=2))"
print $pretty

# External commands in a pipeline
let large_files = ^git ls-files
    | lines
    | each { |f| { name: $f, ext: ($f | path parse | get extension) } }
    | where { $in.ext == "nu" }
print $"Nushell files in repo:"
print $large_files

# Exit code checking
let result = do { ^git status --short } | complete
print $"git status exit code: ($result.exit_code)"
if $result.exit_code == 0 {
    print $"Output: ($result.stdout)"
} else {
    print $"Error: ($result.stderr)"
}
