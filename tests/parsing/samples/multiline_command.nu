# if echo treats both of the below as arguments, it will output a list, which means length should be 2.

echo "first very long argument that necessitates multiline" \
"second long argument that should appear as the second element in a list" | length
