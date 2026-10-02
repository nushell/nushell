let n = 3
[(ls | first --strict=true) ('x' | fill --width=$n) ('x' | fill --width=(1 + 2)) ('x' | fill --character="-")]
