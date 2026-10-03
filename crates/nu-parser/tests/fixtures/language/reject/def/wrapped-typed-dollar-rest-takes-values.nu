# nu looks for `...$rest` followed by `:`, so this rest parameter is typed.
def --wrapped f [...$rest: string] {}
f 'x'$
