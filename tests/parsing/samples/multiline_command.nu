# if echo treats both of the below as arguments, it will output a list.

echo "first very long argument that necessitates multiline" \
"second long argument that should appear as the second element in a list" | length

# below, see what it looks like without the linebreak
echo "arg 1"
"arg 2" | length

# if everything has gone well the whole output of this script will look like:
# 2
# arg 1
# 1
