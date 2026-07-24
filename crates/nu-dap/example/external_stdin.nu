# Externals inherit NUL stdin (not the DAP stream): a child that reads
# stdin must see EOF immediately instead of hanging the session forever.
print "before"
^python -c 'import sys; sys.stdin.read(); print("stdin-drained")'
print "after"
