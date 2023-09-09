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

export def "from ndjson" []: string -> any {
    from json --objects
}

export def "from jsonl" []: string -> any {
    from json --objects
}
