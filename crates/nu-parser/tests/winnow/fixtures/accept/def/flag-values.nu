def --env=true a [] {}
def --env=false --wrapped=true b [...rest] {}
def c --wrapped=false [...rest: int] {}
def --env=(1 == 1) d [] {}
const off = false
def --wrapped=$off e [...rest: int] {}
def --wrapped=false f [--x] {}
