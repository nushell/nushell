overlay use ./foo.nu as prefix --prefix
alias aname = prefix mod name sub module cmd  name  long
aname
prefix foo str
overlay hide prefix

use ./foo.nu [ "mod name" cst_mod ]

$cst_mod."sub module"."sub sub module".var_name
mod name sub module cmd name long
let $cst_mod = 1
$cst_mod
