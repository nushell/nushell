nu -c "help commands | get name | to md | save _names.md"

path="docs/summaries.md"
echo "" > $path
while IFS= read -r command; do
    echo "Appending usage for $command"
    # Keep newlines when using command substition
    RESULTX="$(nu -c "$command --help"; echo x)"
    echo -e "# $command \n" >> $path
    echo -e "${RESULTX%x}" >> $path
done < "_names.md"

rm _names.md
