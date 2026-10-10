([a, b, c] | enumerate | each --keep-empty { |e| if $e.index != 1 { 100 }}).1
