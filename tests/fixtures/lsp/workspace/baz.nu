overlay use ./foo.nu as prefix --prefix
alias aname = prefix mod name sub module cmd name
aname
prefix foo str
overlay hide prefix

use ./foo.nu [ "mod name" cst_mod ]

$cst_mod."sub module".var_name
mod name sub module cmd name
