# In the `else` branch `{}.b` is a cell path on an empty record, as nu reads it as an expression.
if true {} else {}.b
