use module.nu "module name"
overlay use module.nu as new_name
overlay hide --keep-env [PWD] new_name
hide "module name"
