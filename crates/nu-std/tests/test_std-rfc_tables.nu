use std/assert
use std/testing *
use std-rfc/tables *

const test_table = [
  [ col-a     col-b     col-c     col-d     col-e     col-f ];
  [  'a0'      'b0'      'c0'      'd0'      'e0'      'f0' ]
  [  'a1'      'b1'      'c1'      'd1'      'e1'      'f1' ]
  [  'a2'      'b2'      'c2'      'd2'      'e2'      'f2' ]
  [  'a3'      'b3'      'c3'      'd3'      'e3'      'f3' ]
  [  'a4'      'b4'      'c4'      'd4'      'e4'      'f4' ]
  [  'a5'      'b5'      'c5'      'd5'      'e5'      'f5' ]
  [  'a6'      'b6'      'c6'      'd6'      'e6'      'f6' ]
  [  'a7'      'b7'      'c7'      'd7'      'e7'      'f7' ]
  [  'a8'      'b8'      'c8'      'd8'      'e8'      'f8' ]
  [  'a9'      'b9'      'c9'      'd9'      'e9'      'f9' ]
]

const enumerated_table = [
  [ index      col-a     col-b     col-c     col-d     col-e     col-f ];
  [   0         'a0'      'b0'      'c0'      'd0'      'e0'      'f0' ]
  [   1         'a1'      'b1'      'c1'      'd1'      'e1'      'f1' ]
  [   2         'a2'      'b2'      'c2'      'd2'      'e2'      'f2' ]
  [   3         'a3'      'b3'      'c3'      'd3'      'e3'      'f3' ]
  [   4         'a4'      'b4'      'c4'      'd4'      'e4'      'f4' ]
  [   5         'a5'      'b5'      'c5'      'd5'      'e5'      'f5' ]
  [   6         'a6'      'b6'      'c6'      'd6'      'e6'      'f6' ]
  [   7         'a7'      'b7'      'c7'      'd7'      'e7'      'f7' ]
  [   8         'a8'      'b8'      'c8'      'd8'      'e8'      'f8' ]
  [   9         'a9'      'b9'      'c9'      'd9'      'e9'      'f9' ]
]

@test
def select-slice--single_int [] {
  assert equal (
    $test_table | select slices 1
  ) (
    $enumerated_table | select 1
  )
}

@test
def select-slice--single_slice [] {
  assert equal (
    $test_table | select slices 2..4
  ) (
    $enumerated_table | select 2 3 4 
  )
}

@test
def select-slice--complex [] {
  assert equal (
    # First and every following third-row + second row
    $test_table | select slices 1 0..3..100
  ) (
    $enumerated_table | select 0 1 3 6 9
  )
}

@test
def select-slice--out_of_bounds [] {
  assert equal (
    $test_table | select slices 100
  ) (
    []
  )
}

@test
def reject-slice--single_index [] {
  assert equal (
    $test_table | reject slices 4
  ) (
    $enumerated_table | reject 4
  )
}

@test
def reject-slice--slices [] {
  assert equal (
    # Reject rows 0-3 and 5-9, leaving only 4
    $test_table | reject slices 0..3 5..9
  ) (
    $enumerated_table | select 4
  )
}

@test
def reject-slice--out_of_bounds [] {
  assert error {
    $test_table | reject slices 1000
  }
}

@test
def select-col--index [] {
  assert equal (
    $test_table | select column-slices 2
  ) (
    $test_table | select col-c
  )
}

@test
def select-col--indices [] {
  assert equal (
    $test_table | select column-slices 2 4
  ) (
    $test_table | select col-c col-e
  )
}

@test
def select-col--slices_and_index [] {
  assert equal (
    $test_table | select column-slices 0..2..5 1
  ) (
    $test_table | select col-a col-c col-e col-b
  )
}

@test
def reject-col--slices_and_index [] {
  assert equal (
    $test_table | reject column-slices 0..2..5 1
  ) (
    $enumerated_table | select col-d col-f
  )
}

@test
def reject-col--out_of_bounds [] {
  assert equal (
    $test_table | reject column-slices 1_000
  ) (
    $test_table
  )
}

const movies = [
    [ Film, Genre, Lead_Studio, Audience_score_%, Profitability, Rotten_Tomatoes_%, Worldwide_Gross, Year ];
    [ "Youth in Revolt", Comedy, "The Weinstein Company", 52, 1.09, 68, 19.62, 2010 ],
    [ "You Will Meet a Tall Dark Stranger", Comedy, Independent, 35, 1.211818182, 43, 26.66, 2010 ],
    [ "When in Rome", Comedy, Disney, 44, 0, 15, 43.04, 2010 ],
    [ "What Happens in Vegas", Comedy, Fox, 72, 6.267647029, 28, 219.37, 2008 ],
    [ "Water For Elephants", Drama, "20th Century Fox", 72, 3.081421053, 60, 117.09, 2011 ],
    [ WALL-E, Animation, Disney, 89, 2.896019067, 96, 521.28, 2008 ],
    [ Waitress, Romance, Independent, 67, 11.0897415, 89, 22.18, 2007 ],
    [ "Waiting For Forever", Romance, Independent, 53, 0.005, 6, 0.03, 2011 ],
    [ "Valentine's Day", Comedy, "Warner Bros.", 54, 4.184038462, 17, 217.57, 2010 ],
    [ "Tyler Perry's Why Did I get Married", Romance, Independent, 47, 3.7241924, 46, 55.86, 2007 ],
    [ "Twilight: Breaking Dawn", Romance, Independent, 68, 6.383363636, 26, 702.17, 2011 ],
    [ Twilight, Romance, Summit, 82, 10.18002703, 49, 376.66, 2008 ],
    [ "The Ugly Truth", Comedy, Independent, 68, 5.402631579, 14, 205.3, 2009 ],
    [ "The Twilight Saga: New Moon", Drama, Summit, 78, 14.1964, 27, 709.82, 2009 ],
    [ "The Time Traveler's Wife", Drama, Paramount, 65, 2.598205128, 38, 101.33, 2009 ],
    [ "The Proposal", Comedy, Disney, 74, 7.8675, 43, 314.7, 2009 ],
    [ "The Invention of Lying", Comedy, "Warner Bros.", 47, 1.751351351, 56, 32.4, 2009 ],
    [ "The Heartbreak Kid", Comedy, Paramount, 41, 2.129444167, 30, 127.77, 2007 ],
    [ "The Duchess", Drama, Paramount, 68, 3.207850222, 60, 43.31, 2008 ],
    [ "The Curious Case of Benjamin Button", Fantasy, "Warner Bros.", 81, 1.78394375, 73, 285.43, 2008 ],
    [ "The Back-up Plan", Comedy, CBS, 47, 2.202571429, 20, 77.09, 2010 ],
    [ Tangled, Animation, Disney, 88, 1.365692308, 89, 355.01, 2010 ],
    [ "Something Borrowed", Romance, Independent, 48, 1.719514286, 15, 60.18, 2011 ],
    [ "She's Out of My League", Comedy, Paramount, 60, 2.4405, 57, 48.81, 2010 ],
    [ "Sex and the City Two", Comedy, "Warner Bros.", 49, 2.8835, 15, 288.35, 2010 ],
    [ "Sex and the City 2", Comedy, "Warner Bros.", 49, 2.8835, 15, 288.35, 2010 ],
    [ "Sex and the City", Comedy, "Warner Bros.", 81, 7.221795791, 49, 415.25, 2008 ],
    [ "Remember Me", Drama, Summit, 70, 3.49125, 28, 55.86, 2010 ],
    [ "Rachel Getting Married", Drama, Independent, 61, 1.384166667, 85, 16.61, 2008 ],
    [ Penelope, Comedy, Summit, 74, 1.382799733, 52, 20.74, 2008 ],
    [ "P.S. I Love You", Romance, Independent, 82, 5.103116833, 21, 153.09, 2007 ],
    [ "Over Her Dead Body", Comedy, "New Line", 47, 2.071, 15, 20.71, 2008 ],
    [ "Our Family Wedding", Comedy, Independent, 49, 0, 14, 21.37, 2010 ],
    [ "One Day", Romance, Independent, 54, 3.682733333, 37, 55.24, 2011 ],
    [ "Not Easily Broken", Drama, Independent, 66, 2.14, 34, 10.7, 2009 ],
    [ "No Reservations", Comedy, "Warner Bros.", 64, 3.307180357, 39, 92.6, 2007 ],
    [ "Nick and Norah's Infinite Playlist", Comedy, Sony, 67, 3.3527293, 73, 33.53, 2008 ],
    [ "New Year's Eve", Romance, "Warner Bros.", 48, 2.536428571, 8, 142.04, 2011 ],
    [ "My Week with Marilyn", Drama, "The Weinstein Company", 84, 0.8258, 83, 8.26, 2011 ],
    [ "Music and Lyrics", Romance, "Warner Bros.", 70, 3.64741055, 63, 145.9, 2007 ],
    [ "Monte Carlo", Romance, "20th Century Fox", 50, 1.9832, 38, 39.66, 2011 ],
    [ "Miss Pettigrew Lives for a Day", Comedy, Independent, 70, 0.2528949, 78, 15.17, 2008 ],
    [ "Midnight in Paris", Romence, Sony, 84, 8.744705882, 93, 148.66, 2011 ],
    [ "Marley and Me", Comedy, Fox, 77, 3.746781818, 63, 206.07, 2008 ],
    [ "Mamma Mia!", Comedy, Universal, 76, 9.234453864, 53, 609.47, 2008 ],
    [ "Mamma Mia!", Comedy, Universal, 76, 9.234453864, 53, 609.47, 2008 ],
    [ "Made of Honor", Comdy, Sony, 61, 2.64906835, 13, 105.96, 2008 ],
    [ "Love Happens", Drama, Universal, 40, 2.004444444, 18, 36.08, 2009 ],
    [ "Love & Other Drugs", Comedy, Fox, 55, 1.817666667, 48, 54.53, 2010 ],
    [ "Life as We Know It", Comedy, Independent, 62, 2.530526316, 28, 96.16, 2010 ],
    [ "License to Wed", Comedy, "Warner Bros.", 55, 1.9802064, 8, 69.31, 2007 ],
    [ "Letters to Juliet", Comedy, Summit, 62, 2.639333333, 40, 79.18, 2010 ],
    [ "Leap Year", Comedy, Universal, 49, 1.715263158, 21, 32.59, 2010 ],
    [ "Knocked Up", Comedy, Universal, 83, 6.636401848, 91, 219, 2007 ],
    [ Killers, Action, Lionsgate, 45, 1.245333333, 11, 93.4, 2010 ],
    [ "Just Wright", Comedy, Fox, 58, 1.797416667, 45, 21.57, 2010 ],
    [ "Jane Eyre", Romance, Universal, 77, 0, 85, 30.15, 2011 ],
    [ "It's Complicated", Comedy, Universal, 63, 2.642352941, 56, 224.6, 2009 ],
    [ "I Love You Phillip Morris", Comedy, Independent, 57, 1.34, 71, 20.1, 2010 ],
    [ "High School Musical 3: Senior Year", Comedy, Disney, 76, 22.91313646, 65, 252.04, 2008 ],
    [ "He's Just Not That Into You", Comedy, "Warner Bros.", 60, 7.1536, 42, 178.84, 2009 ],
    [ "Good Luck Chuck", Comedy, Lionsgate, 61, 2.36768512, 3, 59.19, 2007 ],
    [ "Going the Distance", Comedy, "Warner Bros.", 56, 1.3140625, 53, 42.05, 2010 ],
    [ "Gnomeo and Juliet", Animation, Disney, 52, 5.387972222, 56, 193.97, 2011 ],
    [ "Gnomeo and Juliet", Animation, Disney, 52, 5.387972222, 56, 193.97, 2011 ],
    [ "Ghosts of Girlfriends Past", Comedy, "Warner Bros.", 47, 2.0444, 27, 102.22, 2009 ],
    [ "Four Christmases", Comedy, "Warner Bros.", 52, 2.022925, 26, 161.83, 2008 ],
    [ Fireproof, Drama, Independent, 51, 66.934, 40, 33.47, 2008 ],
    [ Enchanted, Comedy, Disney, 80, 4.005737082, 93, 340.49, 2007 ],
    [ "Dear John", Drama, Sony, 66, 4.5988, 29, 114.97, 2010 ],
    [ Beginners, Comedy, Independent, 80, 4.471875, 84, 14.31, 2011 ],
    [ "Across the Universe", romance, Independent, 84, 0.652603178, 54, 29.37, 2007 ],
    [ "A Serious Man", Drama, Universal, 64, 4.382857143, 89, 30.68, 2009 ],
    [ "A Dangerous Method", Drama, Independent, 89, 0.44864475, 79, 8.97, 2011 ],
    [ "27 Dresses", Comedy, Fox, 71, 5.3436218, 40, 160.31, 2008 ],
    [ "(500) Days of Summer", comedy, Fox, 81, 8.096, 87, 60.72, 2009 ]
]

@test
def count_movies_by_Lead_Studio [] {
    let grouped = $movies | group-by Lead_Studio --to-table
    let out = $grouped | aggregate
    # let expected = $grouped | insert count {get items | length} | select Lead_Studio count
    let expected = [
        [ Lead_Studio, count ];
        [ "The Weinstein Company", 2 ],
        [ Independent, 19 ],
        [ Disney, 8 ],
        [ Fox, 6 ],
        [ "20th Century Fox", 2 ],
        [ "Warner Bros.", 14 ],
        [ Summit, 5 ],
        [ Paramount, 4 ],
        [ CBS, 1 ],
        [ "New Line", 1 ],
        [ Sony, 4 ],
        [ Universal, 8 ],
        [ Lionsgate, 2 ]
    ]

    assert equal $out $expected
}

@test
def average_gross_by_Genre [] {
    let grouped = $movies | group-by Genre --to-table
    let out = $grouped | aggregate --ops {avg: {math avg}} Worldwide_Gross | select Genre Worldwide_Gross_avg
    # let expected = $grouped | insert Worldwide_Gross_avg {get items.Worldwide_Gross | math avg} | select Genre Worldwide_Gross_avg

    # Round to 2 digits of precision to keep floating point operations consistent between platforms.
    let out = $out | update Worldwide_Gross_avg {math round --precision 2}
    let expected = [
        [ Genre, Worldwide_Gross_avg ];
        [ Comedy, 148.33 ],
        [ Drama, 99.01 ],
        [ Animation, 316.06 ],
        [ Romance, 148.60 ],
        [ Fantasy, 285.43 ],
        [ Romence, 148.66 ],
        [ Comdy, 105.96 ],
        [ Action, 93.40 ],
        [ romance, 29.37 ],
        [ comedy, 60.72 ]
    ]

    assert equal $out $expected
}

@test
def aggregate_default_ops [] {
    let grouped = $movies | group-by Genre --to-table
    let out = $grouped | aggregate Worldwide_Gross

    # Round to 2 digits of precision to keep floating point operations consistent between platforms.
    let out = $out | update cells -c [Worldwide_Gross_min, Worldwide_Gross_avg, Worldwide_Gross_max, Worldwide_Gross_sum] { math round --precision 2 }

    let expected = [
        [Genre    , count, Worldwide_Gross_min, Worldwide_Gross_avg, Worldwide_Gross_max, Worldwide_Gross_sum];
        [Comedy   ,    41,               14.31,              148.33,              609.47,             6081.73],
        [Drama    ,    13,                8.26,               99.01,              709.82,             1287.15],
        [Animation,     4,              193.97,              316.06,              521.28,             1264.23],
        [Romance  ,    12,                0.03,              148.60,              702.17,             1783.16],
        [Fantasy  ,     1,              285.43,              285.43,              285.43,              285.43],
        [Romence  ,     1,              148.66,              148.66,              148.66,              148.66],
        [Comdy    ,     1,              105.96,              105.96,              105.96,              105.96],
        [Action   ,     1,               93.40,               93.40,               93.40,               93.40],
        [romance  ,     1,               29.37,               29.37,               29.37,               29.37],
        [comedy   ,     1,               60.72,               60.72,               60.72,               60.72],
    ]

    assert equal $out $expected
}

@test
def throw_error_on_non-table_input [] {
    # without --to-table
    let out = try {
        $movies | group-by Genre | aggregate Worldwide_Gross
    } catch {|e|
        $e.msg
    }

    assert equal $out "input must be a table"
}

@test
def throw_error_on_non-existing_column [] {
    let grouped = $movies | group-by Genre --to-table
    let error = try {
        $grouped | aggregate --ops {avg: {math avg}} NotInTheDataSet
    } catch {|e|
        $e.json | from json
    }

    assert equal $error.inner.0.msg "Cannot find column '$.items.NotInTheDataSet'"
}

@test
def aggregate_stats_without_grouping [] {
    let out = $movies | aggregate Year | update cells -c [Year_min Year_avg Year_max Year_sum] {math round -p 2}
    let expected = [{
        count: 76,
        Year_min: 2007,
        Year_avg: 2009.09,
        Year_max: 2011,
        Year_sum: 152691
    }]

    assert equal $out $expected
}
