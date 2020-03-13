# calc

calc is a command that takes a math expression from the pipeline and calculates that into a number.

This command supports the following operations -

operations :
* binary operators: +, -, *, /, % (remainder), ^ (power)
* unary operators: +, -, ! (factorial)

functions :
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

## Examples -

```
> echo "1+2+3" | calc
6.000000000000000
> echo "1-2+3" | calc
2.000000000000000
> echo "-(-23)" | calc
23.00000000000000
> echo "5^2" | calc
25.00000000000000
> echo "5^3" | calc
125.0000000000000
> echo "min(5,4,3,2,1,0,-100,45)" | calc
-100.0000000000000
> echo "max(5,4,3,2,1,0,-100,45)" | calc
45.00000000000000
> echo "sqrt(2) | calc"
1.414213562373095
> echo pi | calc
3.141592653589793
> echo e | calc
2.718281828459045
> echo "sin(pi / 2)" | calc
1.000000000000000
> echo "floor(5999/1000)" | calc
5.000000000000000
```

```
❯ open abc.json
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

❯ open abc.json | format "({size} + 500) * 4"
───┬──────────────────
 # │ <value>
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

❯ open abc.json | format "({size} + 500) * 4" | calc
───┬───────────
 # │ <value>
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

❯ open abc.json | format "({size} - 1000) * 4" | calc
───┬────────────
 # │ <value>
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

Note that since `calc` uses floating-point numbers, the result may not always be precise.

```
> echo "floor(5999999999999999999/1000000000000000000)" | calc
6.000000000000000
```
