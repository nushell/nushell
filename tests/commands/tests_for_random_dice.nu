use std assert

# Parameter name:
# sig type   : nothing
# name       : dice
# type       : named
# shape      : int
# description: The amount of dice being rolled

# Parameter name:
# sig type   : nothing
# name       : sides
# type       : named
# shape      : int
# description: The amount of sides a die has


# This is the custom command 1 for random_dice:

#[test]
def random_dice_roll_1_dice_with_6_sides_each_1 [] {
  let result = (random dice)
  assert ($result == )
}

# This is the custom command 2 for random_dice:

#[test]
def random_dice_roll_10_dice_with_12_sides_each_2 [] {
  let result = (random dice -d 10 -s 12)
  assert ($result == )
}


