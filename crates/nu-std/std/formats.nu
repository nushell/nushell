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
