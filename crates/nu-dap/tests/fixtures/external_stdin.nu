# Externals inherit NUL stdin (not the DAP stream): a child that reads
# stdin must see EOF immediately instead of hanging the session forever.
# A bare `python` is missing on some CI images (macOS), so take whichever exists.
print "before"
let py = (which python python3 | get path.0)
^$py -c 'import sys; sys.stdin.read(); print("stdin-drained")'
print "after"
