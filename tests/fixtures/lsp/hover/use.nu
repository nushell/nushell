use cell_path.nu [ r foo ]
def test [] {
$r.foo.1.bar
let foo = [{a: {b: [1]}}]
$foo.a.b.0.0
}
