# String processing: interpolation, splitting, regex, and transforms.
# Watch string values change as you step through each operation.

let raw = "  Hello, World!  Nushell is great.  "

# Trimming and basic transforms
let trimmed = $raw | str trim
let upper = $trimmed | str upcase
let lower = $trimmed | str downcase
print $"Trimmed: '($trimmed)'"
print $"Upper: ($upper)"

# Splitting and joining
let csv_line = "alice,30,engineer,NYC"
let fields = $csv_line | split row ","
print $"Fields: ($fields)"
print $"Name: ($fields | get 0)"

let rejoined = $fields | str join " | "
print $"Rejoined: ($rejoined)"

# String contains / starts-with / ends-with
let filename = "report_2024_final.pdf"
let is_pdf = $filename | str ends-with ".pdf"
let is_report = $filename | str starts-with "report"
let has_year = $filename | str contains "2024"
print $"($filename): pdf=($is_pdf) report=($is_report) has_year=($has_year)"

# Substring and replace
let sentence = "The quick brown fox jumps over the lazy dog"
let replaced = $sentence | str replace "fox" "cat"
let replaced_all = $sentence | str replace --all "the" "a"
print $"Replace: ($replaced)"
print $"Replace all: ($replaced_all)"

# Regex matching
let log_line = "2024-03-15 14:23:45 [ERROR] Connection timeout after 30s"
let has_error = $log_line | str contains "ERROR"
let parts = $log_line | parse "{date} {time} [{level}] {message}"
print $parts

# Building strings from data
let users = [
    { name: "Alice", role: "admin" }
    { name: "Bob", role: "user" }
    { name: "Carol", role: "user" }
]
let user_list = $users | each { |u|
    $"• ($u.name) \(($u.role))"
} | str join "\n"
print $user_list

# Multi-line string (heredoc-style)
let template = $"
Server Report
=============
Host: ($raw | str trim)
Users: ($users | length)
Status: OK
"
print $template
