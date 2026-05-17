$env.int = 1
$env.float = 1.1
$env.table = [[]; []]
$env.list ++= []
$env.record = {}

mut foo = 1kb
let bar = 1kb
$foo += $bar
