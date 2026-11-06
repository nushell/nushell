module m { export def f [] { 1 } }
use m
use m f
use m [f]
use m *
m f
