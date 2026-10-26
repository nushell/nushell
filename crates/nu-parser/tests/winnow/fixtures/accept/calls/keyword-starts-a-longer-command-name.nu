# `if`, `return`, ... are commands for nu: a longer command name that starts with one is
# called instead.
def "if ready" [then: closure] { do $then }
if ready { 'ran' }
def "return early" [] { 1 }
def g [] { return early }
