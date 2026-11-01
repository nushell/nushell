# Long-running fixture for the tests that interrupt a *running* script
# (`pause`, and `restart` outside a breakpoint). Each iteration is slow enough
# that a request can land mid-run, and prints its number so a test can tell one
# run's output from another's.
for i in 1..20 {
    sleep 100ms
    print $"tick ($i)"
}
print "done"
