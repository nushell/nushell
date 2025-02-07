use std/assert
use std-rfc/str

#[test]
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
    ) "Heading\n\n    one\n    two"
}

#[test]
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
    ) "Heading\n                \n    one\n    two"
}

#[test]
def str-dedent_leave_blankline_tab [] {
    # Test 3:
    # Same, but with a single tab character on the "blank" line
    assert equal (
        do {
            let s = "   
                Heading
\t
                    one
                    two
                "
            $s | str dedent
        }
    ) "Heading\n\t\n    one\n    two"
}

#[test]
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
    ) "Heading\n\n    one\n    two\n"
}

#[test]
def str-dedent_indentity [] {
    # Test 5:
    # Identity - Returns the original string sans first and last empty lines
    # No other whitespace should be removed
    assert equal (
        do {
            let s = "\n  Identity  \n"
            $s | str dedent
        }
    ) "  Identity  "
}

#[test]
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
        let s = "Error\n \nTesting\n"
        $s | str dedent
    }
}

#[test]
def str-dedent_error-no_blank_first_line [] {
    # Test 7:
    # Error - Does not contain an empty last line
    assert error {||
        let s = "
            Error"
        $s | str dedent
    }
}

#[test]
def str-dedent_error-missing_last_empty_line [] {
    # Test 7.1:
    # Error - Does not contain an empty last line
    assert error {||
        let s = "

            Error"
        $s | str dedent
    }
}

#[test]
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

#[test]
def str-dedent_error-not_enough_indentation2 [] {
    # Test 8:
    # Error - Line 2 does not have enough indentation
    assert error {||
        let s = "   
            Line 1
           Line 2
            "
        $s | str dedent
    }

    # Test 9:
    # Error - Line does not have enough indentation
    assert error {||
        let s = "   
           Line  
            "
        $s | str dedent
    }
}

#[test]
def str-dedent_first_line_whitespace_allowed [] {
    # Test 10:
    # "Hidden" whitespace on the first line is allowed
    assert equal (
        do {
            let s = "   \t \n  Identity  \n"
            $s | str dedent
        }
    ) "  Identity  "
}

#[test]
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

    let expected = "Heading\n\n    one\n    two"

    assert equal $actual $expected
}

#[test]
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

#[test]
def str-unindent_keep_extra_line [] {
  # Test 3:
  # Keep intentional blank lines at start and/or end

  let actual = "

  Content

  " | str unindent

  let expected = $"(char newline)Content(char newline)"

  assert equal $actual $expected
}

#[test]
def str-unindent_works_on_single_line [] {
    # Test 4:
    # Works on a single-line string
    # And trailing whitespace is preserved

    let actual = ("    Content  " | str unindent)
    let expected = "Content  "

    assert equal $actual $expected
}

#[test]
def str-unindent_whitespace_only_single_line [] {
    # Test 4:
    # Works on a single-line string with whitespace-only
    # Returns the original string

    let actual = ("   " | str unindent)
    let expected = "   "

    assert equal $actual $expected
}
    
#[test]
def str-unindent_whitespace_works_with_tabs [] {
    # Test 4:
    # Works with tabs for indentation

    let actual = (
        $"(char newline)(char tab)(char tab)Content(char newline)"
        | str unindent --tab
    )

    let expected = $"Content"

    assert equal $actual $expected
}
