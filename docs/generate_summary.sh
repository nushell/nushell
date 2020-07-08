nu -c "help commands | get name | to md | trim | save _names.md"

path="docs/summaries.md"
echo "" > $path
while IFS= read -r command; do
    echo "Appending usage for $command"
    # Keep newlines when using command substition and sed removes colour codes
    RESULTX="$(nu -c "$command --help" | sed -r "s/\x1B\[([0-9]{1,3}(;[0-9]{1,2})?)?[mGK]//g"; echo x)"
    echo -e "# $command \n" >> $path
    echo -e "${RESULTX%x}" >> $path
done < "_names.md"

rm _names.md
