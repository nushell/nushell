use std/assert greater

# for examples
alias "random dice" = dice

# Generate a random dice roll
@search-terms "generate" "die" "1-6"
@example "Roll 1 dice with 6 sides each" { random dice }
@example "Roll 10 dice with 12 sides each" {
    random dice --dice 10 --sides 12
}
export def dice [
    --dice = 1  # The amount of dice being rolled
    --sides = 6  # The amount of sides a die has
]: nothing -> list<int> {
    greater $dice 0 "The amount of dice must be greater than 0"
    greater $sides 1 "The amount of sides must be greater than 1"

    mut out = []
    for _ in 1..$dice {
    	$out ++= [(random int 1..$sides)]
    }
    $out
}
