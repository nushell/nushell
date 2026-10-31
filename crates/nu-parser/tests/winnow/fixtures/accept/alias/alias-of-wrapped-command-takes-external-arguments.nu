# An alias of a wrapped command takes external arguments too.
def --wrapped f [...rest] {}
alias g = f -e
g 'x'$
