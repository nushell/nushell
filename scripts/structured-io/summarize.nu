#!/usr/bin/env nu
# only works with a structured-io-aware nu

# Aggregates a host table from the previous script. Disk stays a filesize.

def main [] {
    $in
    | group-by region --to-table
    | each {|row|
        {
            region: $row.region
            hosts: ($row.items | length)
            disk: ($row.items | get disk | math sum)
            up: ($row.items | where status == up | length)
        }
    }
}
