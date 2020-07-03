# shuffle

Shuffles the rows in a random order.

## Examples

Passing the same input to shuffle multiple times gives different results -

```shell
> echo [ a b c d ] | shuffle
───┬───
 0 │ a
 1 │ c
 2 │ d
 3 │ b
───┴───
```

```shell
> echo [ a b c d ] | shuffle
───┬───
 0 │ c
 1 │ b
 2 │ d
 3 │ a
───┴───
```

```shell
> echo [ a b c d ] | shuffle
───┬───
 0 │ c
 1 │ b
 2 │ a
 3 │ d
───┴───
```
