#!/usr/bin/env nu

# Standalone script. Returns a typed table, not text.
# With structured-io, the parent can `get`, `where`, `math` on this.

def main [] {
    [
        [host        region   role      disk  uptime last_patch           status];
        [ferris      us-west  builder   1tb   14day  2026-08-01           up]
        [nugget      us-west  runner    800gb 3hr    2026-08-18           up]
        [reedline    eu-central ci      2tb   41day  2026-06-02           down]
        [polars      eu-central worker  4tb   6day   2026-08-12           up]
        [byte-stream ap-east  cache     500gb 22hr   2026-08-21           down]
        [nuon        us-east  api       1tb   9day   2026-07-30           up]
    ]
}
