mut foo = 'bar'
$foo = $foo | str upcase | str reverse
$foo ++= $foo | str upcase
