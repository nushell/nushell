#!/usr/bin/env nu

# Pipeline filter. `$in` is a table when the parent is nu with structured-io.

def main [] {
    $in | where status == up
}
