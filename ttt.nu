#!/usr/bin/env nu

# Initialize empty board with dots for better visibility
def create_board [] {
    [
        ["·", "·", "·"],
        ["·", "·", "·"],
        ["·", "·", "·"]
    ]
}

# Display the board using simple formatting
def display_board [board] {
    print ""
    print "    a   b   c"
    print "  ┌───┬───┬───┐"
    
    for $i in 0..2 {
        let row = $board | get $i
        let row_num = $i + 1
        print $"($row_num) │ ($row.0) │ ($row.1) │ ($row.2) │"
        if $i < 2 {
            print "  ├───┼───┼───┤"
        }
    }
    print "  └───┴───┴───┘"
    print ""
}

# Show coordinate reference
def show_coordinates [] {
    print "Coordinate reference:"
    print "    a   b   c"
    print "  ┌───┬───┬───┐"
    print "1 │1a │1b │1c │"
    print "  ├───┼───┼───┤"
    print "2 │2a │2b │2c │"
    print "  ├───┼───┼───┤"
    print "3 │3a │3b │3c │"
    print "  └───┴───┴───┘"
    print "Enter coordinates like: 1a, 2b, 3c, a1, b2, c3"
    print ""
}

# Parse coordinate input (accepts formats like "1a", "2b", "a1", "b2", etc.)
def parse_coordinates [input] {
    # Convert to lowercase and remove spaces/separators
    let clean_input = ($input | str downcase | str replace --all --regex '[^a-z0-9]' '')
    
    if ($clean_input | str length) != 2 {
        return null
    }
    
    let chars = ($clean_input | split chars)
    mut row_num = 0
    mut col_num = 0
    mut found_row = false
    mut found_col = false
    
    # Parse each character as either row (1-3) or column (a-c)
    for $char in $chars {
        if $char in ["1", "2", "3"] {
            if not $found_row {
                try {
                    $row_num = ($char | into int)
                    $found_row = true
                } catch {
                    return null
                }
            } else {
                return null  # Two row numbers found
            }
        } else if $char in ["a", "b", "c"] {
            if not $found_col {
                $col_num = (match $char { 
                    "a" => 1, 
                    "b" => 2, 
                    "c" => 3,
                    _ => 0
                })
                $found_col = true
            } else {
                return null  # Two column letters found
            }
        } else {
            return null  # Invalid character
        }
    }
    
    # Check if we found both row and col and they're in valid range
    if $found_row and $found_col and ($row_num >= 1) and ($row_num <= 3) and ($col_num >= 1) and ($col_num <= 3) {
        return {row: ($row_num - 1), col: ($col_num - 1)}  # Convert to 0-based indexing
    }
    
    return null
}

# Check if a coordinate is valid and empty
def is_valid_coordinate [board, row, col] {
    let cell = $board | get $row | get $col
    $cell == "·"
}

# Make a move on the board using coordinates
def make_move [board, row, col, player] {
    $board | update $row {|r| $r | update $col $player}
}

# Check for winner
def check_winner [board] {
    # Check rows
    for $row in $board {
        if ($row.0 == $row.1) and ($row.1 == $row.2) and ($row.0 != "·") {
            return $row.0
        }
    }
    
    # Check columns
    for $col in 0..2 {
        let col_vals = $board | each {|row| $row | get $col}
        if ($col_vals.0 == $col_vals.1) and ($col_vals.1 == $col_vals.2) and ($col_vals.0 != "·") {
            return $col_vals.0
        }
    }
    
    # Check diagonals
    let diag1 = [($board.0.0), ($board.1.1), ($board.2.2)]
    if ($diag1.0 == $diag1.1) and ($diag1.1 == $diag1.2) and ($diag1.0 != "·") {
        return $diag1.0
    }
    
    let diag2 = [($board.0.2), ($board.1.1), ($board.2.0)]
    if ($diag2.0 == $diag2.1) and ($diag2.1 == $diag2.2) and ($diag2.0 != "·") {
        return $diag2.0
    }
    
    return null
}

# Check if board is full
def is_board_full [board] {
    $board | flatten | all {|cell| $cell != "·"}
}

# Get player input
def get_player_move [player] {
    loop {
        let input = (input $"Player ($player), enter coordinates: ")
        
        let coords = (parse_coordinates $input)
        if $coords != null {
            return $coords
        } else {
            print "Please enter valid coordinates like: 1a, 2b, 3c, a1, b2, or c3"
        }
    }
}

# Main game function
def play_game [] {
    mut board = (create_board)
    mut current_player = "X"
    mut game_over = false
    
    print "Welcome to Tic-Tac-Toe!"
    print "Players take turns placing X and O"
    show_coordinates
    
    while not $game_over {
        display_board $board
        
        let coords = (get_player_move $current_player)
        
        if (is_valid_coordinate $board $coords.row $coords.col) {
            $board = (make_move $board $coords.row $coords.col $current_player)
            
            let winner = (check_winner $board)
            if $winner != null {
                display_board $board
                print $"🎉 Player ($winner) wins!"
                $game_over = true
            } else if (is_board_full $board) {
                display_board $board
                print "It's a tie! 🤝"
                $game_over = true
            } else {
                # Switch players
                $current_player = (if $current_player == "X" { "O" } else { "X" })
            }
        } else {
            print "Invalid move! Coordinates must be empty and between 1a and 3c"
        }
    }
    
    let play_again = (input "Play again? y/n: ")
    if $play_again =~ "(?i)^y" {
        play_game
    } else {
        print "Thanks for playing! 👋"
    }
}

# Start the game
play_game
