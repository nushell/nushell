# Externals inherit NUL stdin (not the DAP stream): a child that reads
# stdin must see EOF immediately instead of hanging the session forever.
print "before"
let testbin = $env.NUSHELL_TEST_INPUT_BYTES_LENGTH
^$testbin
print "after"
