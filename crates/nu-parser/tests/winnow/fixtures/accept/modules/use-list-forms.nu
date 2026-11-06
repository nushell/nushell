module m { export def f [] { 1 }; export def g [] { 2 } }
use m [f g]
use m [f, g]
use m [
  f
  g
]
