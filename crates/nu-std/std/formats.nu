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

# Convert from NDJSON to structured data.
export def "from ndjson" []: string -> any {
    from json --objects
}

# Convert from JSONL to structured data.
export def "from jsonl" []: string -> any {
    from json --objects
}

# Convert structured data to NDJSON.
def "to ndjson" []: any -> string {
    each { to json --raw } | to text
}

# Convert structured data to JSONL.
def "to jsonl" []: any -> string {
    each { to json --raw } | to text
}
