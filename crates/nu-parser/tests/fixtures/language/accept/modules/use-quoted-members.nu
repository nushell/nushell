module m { export def "sub cmd" [] { 1 } }
use m "sub cmd"
use m ["sub cmd"]
