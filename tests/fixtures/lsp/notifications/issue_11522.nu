#!/usr/bin/env nu

# Important to reproduce the crash
# use a non ascii char somewhere in comments: è

[a b c d] | where {
