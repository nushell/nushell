module m {
    export type T = int
    type U = string
}
use m T
def f [x: T] { $x }
