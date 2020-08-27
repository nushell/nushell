# math eval

math eval is a command that takes a math expression from the pipeline and evaluates that into a number. It also optionally takes the math expression as an argument.

This command supports the following operations -

operations:

* Binary operators: +, -, *, /, % (remainder), ^ (power)
* Unary operators: +, -, ! (factorial)

functions:

* sqrt, abs
* exp, ln, log10
* sin, cos, tan, asin, acos, atan, atan2
* sinh, cosh, tanh, asinh, acosh, atanh
* floor, ceil, round
* signum
* max(x, ...), min(x, ...): maximum and minimum of 1 or more numbers

constants:

* pi
* e

## Examples

```shell
> echo "1+2+3" | math eval
6.0u
```

```shell
> echo "1-2+3" | math eval
2.0
```

```shell
> echo "-(-23)" | math eval
23.0
```

```shell
> echo "5^2" | math eval
25.0
```

```shell
> echo "5^3" | math eval
125.0
```

```shell
> echo "min(5,4,3,2,1,0,-100,45)" | math eval
-100.0
```

```shell
> echo "max(5,4,3,2,1,0,-100,45)" | math eval
45.0
```

```shell
> echo sqrt(2) | math eval
1.414213562373095
```

```shell
> echo pi | math eval
3.141592653589793
```

```shell
> echo e | math eval
2.718281828459045
```

```shell
> echo "sin(pi / 2)" | math eval
1.0
```

```shell
> echo "floor(5999/1000)" | math eval
5.0
```

```shell
> open abc.json
───┬──────
 # │ size
───┼──────
 0 │  816
 1 │ 1627
 2 │ 1436
 3 │ 1573
 4 │  935
 5 │   52
 6 │  999
 7 │ 1639
───┴──────
```

```shell
> open abc.json | format "({size} + 500) * 4"
───┬──────────────────
 # │
───┼──────────────────
 0 │ (816 + 500) * 4
 1 │ (1627 + 500) * 4
 2 │ (1436 + 500) * 4
 3 │ (1573 + 500) * 4
 4 │ (935 + 500) * 4
 5 │ (52 + 500) * 4
 6 │ (999 + 500) * 4
 7 │ (1639 + 500) * 4
───┴──────────────────
```

```shell
> open abc.json | format "({size} + 500) * 4" | math eval
───┬───────────
 # │
───┼───────────
 0 │ 5264.0000
 1 │ 8508.0000
 2 │ 7744.0000
 3 │ 8292.0000
 4 │ 5740.0000
 5 │ 2208.0000
 6 │ 5996.0000
 7 │ 8556.0000
───┴───────────
```

```shell
> open abc.json | format "({size} - 1000) * 4" | math eval
───┬────────────
 # │
───┼────────────
 0 │  -736.0000
 1 │  2508.0000
 2 │  1744.0000
 3 │  2292.0000
 4 │  -260.0000
 5 │ -3792.0000
 6 │    -4.0000
 7 │  2556.0000
───┴────────────
```

Note that since `math eval` uses floating-point numbers, the result may not always be precise.

```shell
> echo "floor(5999999999999999999/1000000000000000000)" | math eval
6.0
```
