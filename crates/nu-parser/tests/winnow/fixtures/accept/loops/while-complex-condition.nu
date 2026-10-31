mut i = 0
while $i < 10 and not ($i == 5) { $i += 1; if $i == 3 { break } }
while ($i | into bool) { break }
while true { break }
