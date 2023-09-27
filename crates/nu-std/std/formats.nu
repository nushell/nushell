# formats.nu
#
# This file contains functions for formatting data in various ways.
#
# Usage:
#   use std format *
#   use std format <function name>
#
# These functions help `open` the files with unsupported extensions such as ndjson.
#

# Convert from ndjson to structured data.
export def "from ndjson" []: string -> any {
    from json --objects
}

# Convert from jsonl to structured data.
export def "from jsonl" []: string -> any {
    from json --objects
}

# Convert structured data to ndjson.
def "to ndjson" []: any -> string {
    each { to json --raw } | to text
}

# Convert structured data to jsonl.
def "to jsonl" []: any -> string {
    each { to json --raw } | to text
}
