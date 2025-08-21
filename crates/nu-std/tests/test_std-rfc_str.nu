use std/assert
use std/testing *
use std-rfc/str

@test
def str-dedent_simple [] {
    
    # Test 1:
    # Should start with "Heading" in the first character position
    # Should not end with a line-break
    # The blank line has no extra spaces
    assert equal (
        do {
            let s = "   
                Heading

                    one
                    two
                "
            $s | str dedent
        }
    ) $"Heading(char lsep)(char lsep)    one(char lsep)    two"
}

@test
def str-dedent_leave_blankline_whitespace [] {
    # Test 2:
    # Same as #1, but the blank line has leftover whitespace
    # indentation (16 spaces) which is left in the result
    assert equal (
        do {
            let s = "   
                Heading
                
                    one
                    two
                "
            $s | str dedent
        }
    ) $"Heading(char lsep)                (char lsep)    one(char lsep)    two"
}

@test
def str-dedent_leave_blankline_tab [] {
    # Test 3:
    # Same, but with a single tab character on the "blank" line
    assert equal (
        do {
            let s = $"   
                Heading
(char tab)
                    one
                    two
                "
            $s | str dedent
        }
    ) $"Heading(char lsep)(char tab)(char lsep)    one(char lsep)    two"
}

@test
def str-dedent_ends_with_newline [] {
    # Test 4:
    # Ends with line-break
    assert equal (
        do {
            let s = "   
                Heading

                    one
                    two

                "
            $s | str dedent
        }
    ) $"Heading(char lsep)(char lsep)    one(char lsep)    two(char lsep)"
}

@test
def str-dedent_identity [] {
    # Test 5:
    # Identity - Returns the original string sans first and last empty lines
    # No other whitespace should be removed
    assert equal (
        do {
            let s = $"(char lsep)  Identity  (char lsep)"
            $s | str dedent
        }
    ) "  Identity  "
}

@test
def str-dedent_error-no_blank_lines [] {
    # Test 6:
    # Error - Does not contain an empty first line
    assert error {||
        let s = "Error"
        $s | str dedent
    }

    # Test 6.1:
    # Error - Does not contain an empty first line
    assert error {||
        let s = $"Error(char lsep) (char lsep)Testing(char lsep)"
        $s | str dedent
    }
}

@test
def str-dedent_error-no_blank_first_line [] {
    # Test 7:
    # Error - Does not contain an empty last line
    assert error {||
        let s = "
            Error"
        $s | str dedent
    }
}

@test
def str-dedent_error-missing_last_empty_line [] {
    # Test 7.1:
    # Error - Does not contain an empty last line
    assert error {||
        let s = "

            Error"
        $s | str dedent
    }
}

@test
def str-dedent_error-not_enough_indentation [] {
    # Test 8:
    # Error - Line 1 does not have enough indentation
    assert error {||
        let s = "   
           Line 1
            Line 2
            "
        $s | str dedent
    }
}

@test
def str-dedent_error-not_enough_indentation2 [] {
    # Test 8.1:
    # Error - Line 2 does not have enough indentation
    assert error {||
        let s = "   
            Line 1
           Line 2
            "
        $s | str dedent
    }
}

@test
def str-dedent_error-not_enough_indentation3 [] {
    # Test 8.2:
    # Error - Line does not have enough indentation
    assert error {||
        let s = "   
           Line  
            "
        $s | str dedent
    }
}

@test
def str-dedent_first_line_whitespace_allowed [] {
    # Test 9:
    # "Hidden" whitespace on the first line is allowed
    assert equal (
        do {
            let s = $"   (char tab) (char lsep)  Identity  (char lsep)"
            $s | str dedent
        }
    ) "  Identity  "
}

@test
def str-dedent_using_tabs [] {
    # Test 10:
    # If the indentation on the last line uses tabs, then the number of tabs
    # will be used instead of spaces
    let actual = (
        $"(char lsep)(char tab)(char tab)First line(char lsep)(char tab)(char tab)(char tab)Second line(char lsep)(char tab)(char tab)"
        | str dedent
    )

    let expected = $"First line(char lsep)(char tab)Second line"

    assert equal $actual $expected
}

@test
def str-unindent_simple [] {
    # Test 1:
    # Should start with "Heading" in the first character position
    # Should not end with a line-break
    # The blank line has no extra spaces
    let actual = (
        "   
            Heading

                one
                two
        "
        | str unindent
    )

    let expected = $"Heading(char lsep)(char lsep)    one(char lsep)    two"

    assert equal $actual $expected
}

@test
def str-unindent_ignore_first_and_last_whitespace [] {
    # Test 2:
    # If the first and/or last line are only whitespace
    # then they shouldn't be included in the result

    let actual = "   
            Heading

                one
                two
        "
        | str unindent

    let expected = "            Heading

                one
                two"
        | str unindent

    assert equal $actual $expected
}

@test
def str-unindent_keep_extra_line [] {
  # Test 3:
  # Keep intentional blank lines at start and/or end

  let actual = "

  Content

  " | str unindent

  let expected = $"(char lsep)Content(char lsep)"

  assert equal $actual $expected
}

@test
def str-unindent_works_on_single_line [] {
    # Test 4:
    # Works on a single-line string
    # And trailing whitespace is preserved

    let actual = ("    Content  " | str unindent)
    let expected = "Content  "

    assert equal $actual $expected
}

@test
def str-unindent_whitespace_only_single_line [] {
    # Test 4:
    # Works on a single-line string with whitespace-only
    # Returns the original string

    let actual = ("   " | str unindent)
    let expected = "   "

    assert equal $actual $expected
}
    
@test
def str-unindent_whitespace_works_with_tabs [] {
    # Test 4:
    # Works with tabs for indentation

    let actual = (
        $"(char lsep)(char tab)(char tab)Content(char lsep)"
        | str unindent --tabs
    )

    let expected = "Content"

    assert equal $actual $expected
}

@test
def str-align_simple [] {
    let actual = [
        "let a = 1"
        "let max = 2"
        "let very_long_variable_name = 3"
    ] | str align '='

    let expected = [
        "let a                       = 1"
        "let max                     = 2"
        "let very_long_variable_name = 3"
    ] | str join "\n"

    assert equal $actual $expected
}

@test
def str-align_center [] {
    let actual = [
        "a = 1"
        "max = 2"
        "very_long_variable_name = 3"
    ] | str align '=' --center

    let expected = [
        "                      a = 1"
        "                    max = 2"
        "very_long_variable_name = 3"
    ] | str join "\n"

    assert equal $actual $expected
}

@test
def str-align_with_range [] {
    let actual = r#'match 5 {
    1.. => { print "More than zero" }
    0 => { print "Zero" }
    -1 => { print "Negative one" }
    -119283 => { print "Very negative" }
}'# | str align '=>' --range 2..

    let expected = r#'match 5 {
    1.. => { print "More than zero" }
    0       => { print "Zero" }
    -1      => { print "Negative one" }
    -119283 => { print "Very negative" }
}'# | lines | str join "\n"

    assert equal $actual $expected
}

@test
def str-align_ignore_lines_with_no_target [] {
    let actual = [
        "let a = 1"
        "let max = 2"
        "# comment"
    ] | str align '='

    let expected = [
        "let a   = 1"
        "let max = 2"
        "# comment"
    ] | str join "\n"

    assert equal $actual $expected
}

@test
def str-align_use_different_char [] {
    let actual = [
        "=>"
        "=====>"
    ] | str align '>' -c '='

    let expected = [
        "=====>"
        "=====>"
    ] | str join "\n"

    assert equal $actual $expected
}

@test
def str-align_multiple_target_in_line [] {
    let actual = [
        "print test # Hello # World"
        "print hello there # test"
    ] | str align '#'

    let expected = [
        "print test        # Hello # World"
        "print hello there # test"
    ] | str join "\n"

    assert equal $actual $expected
}

@test
def str-align_no_target [] {

    let expected = [
        "print test # Hello # World"
        "print hello there # test"
    ] | str join "\n"

    let actual = $expected | str align '='

    assert equal $actual $expected
}

@test
def str-align_empty_target_noop [] {

    let expected = [
        "print test # Hello # World"
        "print hello there # test"
    ] | str join "\n"

    let actual = $expected | str align ''

    assert equal $actual $expected
}

@test
def str-align_empty_input_noop [] {

    let expected = ""

    let actual = [] | str align '='

    assert equal $actual $expected
}
