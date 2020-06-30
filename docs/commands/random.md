# random

Use `random` to generate random values

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

## dice

* `random dice`: Generate a random dice roll

### dice Flags

* `d`, `--dice` \<integer>: The amount of dice being rolled
* `s`, `--sides` \<integer>: The amount of sides a die has

### dice Examples

```shell
> random dice
4
```

```shell
> random dice -d 10 -s 12
───┬────
 0 │ 11
 1 │ 11
 2 │ 11
 3 │ 11
 4 │  5
 5 │  3
 6 │ 10
 7 │  7
 8 │  3
 9 │  1
───┴────
```

## uuid

* `random uuid`: Generate a random uuid4 string

### uuid Examples

```shell
> random uuid
8af4de39-acbc-42f0-94d1-7cfad6c01f8b
```
