# Spreading a record literal keeps the type `record`, which is not a block.
if true { a: 1, ...{b: 2} }
