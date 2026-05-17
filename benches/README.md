# Tango benchmarks

These are benchmarks using [tango](https://github.com/bazhenov/tango), a benchmark tool which pairs the executions of two versions of the given benchmarks. This claims to reduce the noise from other factors on the system and allow a more reliable comparison of two different implementation.

The easiest way to start is to run:

```nushell
use toolkit.nu
# To compare the current branch to main
toolkit benchmark-compare
# or to compare a target git-revision against a reference git-revision
toolkit benchmark-compare <target> <reference>
```
