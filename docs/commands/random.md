# random

Use `random` to generate random values

## uuid

* `random uuid`: Generate a random uuid4 string

### uuid Examples

```shell
> random uuid
8af4de39-acbc-42f0-94d1-7cfad6c01f8b
```

## bool

* `random bool`: Generate a random boolean value

### bool Flags

* `-b`, `--bias` \<number>: Adjusts the probability of a "true" outcome

### bool Examples

```shell
> random bool
false
```

```shell
> random bool --bias 0.75
true
```
