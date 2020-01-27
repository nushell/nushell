# calc

calc is a command that takes a math expression as an argument and calculates that into a number.

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
* max(x, ...), min(x, ...): maximum and minimumum of 1 or more numbers

constants:
* pi
* e
 
## Examples - 

```
> calc "1+2+3"
6.000000000000000
> calc "1-2+3"
2.000000000000000
> calc "-(-23)"
23.00000000000000
> calc "5^2"
25.00000000000000
> calc "5^3"
125.0000000000000
> calc "min(5,4,3,2,1,0,-100,45)"
-100.0000000000000
> calc "max(5,4,3,2,1,0,-100,45)"
45.00000000000000
> calc "sqrt(2)"
1.414213562373095
> calc pi
3.141592653589793
> calc e
2.718281828459045
> calc "sin(pi / 2)"
1.000000000000000
> calc "floor(5999/1000)"
5.000000000000000
```

Note that since `calc` uses floating-point numbers, the result may not always be precise. 

```
> calc "floor(5999999999999999999/1000000000000000000)"
6.000000000000000
```
