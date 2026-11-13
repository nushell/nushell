# Every flag upstream `print` accepts, so the debugger's shim can't drift from
# it: a script that runs under `nu` must also parse under `nu --dap`.
print "plain"
print --raw "raw text"
print -r "short raw"
print --no-newline "no newline"
print --raw (0x[68 69])
