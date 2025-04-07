use cell_path.nu [ r foo ]
def test [] {
$r.foo.1.bar
}
