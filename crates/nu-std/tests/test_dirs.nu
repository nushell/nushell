use std/testing *
use std/assert
use std/log

# A couple of nuances to understand when testing module that exports environment:
# Each 'use' for that module in the test script will execute the def --env block.
# PWD at the time of the `use` will be what the export def --env block will see.

@before-each
def before-each [] {
    # need some directories to play with
    let base_path = ($nu.temp-path | path join $"test_dirs_(random uuid)")
    let path_a = ($base_path | path join "a")
    let path_b = ($base_path | path join "b")

    mkdir $base_path $path_a $path_b

    {base_path: $base_path, path_a: $path_a, path_b: $path_b}
}

@after-each
def after-each [] {
    let base_path = $in.base_path
    cd $base_path
    cd ..
    rm -r $base_path
}

def cur_dir_check [expect_dir, scenario] {
    log debug $"check dir ($expect_dir), scenario ($scenario)"
    assert equal $expect_dir $env.PWD $"expected not PWD after ($scenario)"
}

def cur_ring_check [expect_dir:string, expect_position: int scenario:string] {
    log debug $"check ring ($expect_dir), position ($expect_position) scenario ($scenario)"
    assert ($expect_position < ($env.DIRS_LIST | length)) $"ring big enough after ($scenario)"
    assert equal $expect_position $env.DIRS_POSITION $"position in ring after ($scenario)"
}

@test
def dirs_command [] {
    # careful with order of these statements!
    # must capture value of $in before executing `use`s
    let $c = $in

    # must set PWD *before* doing `use` that will run the def --env block in dirs module.
    cd $c.base_path

    # hide existing variables to prevent the state from outside affecting the tests
    hide-env -i DIRS_LIST DIRS_POSITION
    # must execute these uses for the UOT commands *after* the test and *not* just put them at top of test module.
    # the def --env gets messed up
    use std/dirs

    # Stack: [BASE]
    assert equal [$c.base_path] $env.DIRS_LIST "list is just pwd after initialization"

    dirs next
    assert equal $c.base_path $env.DIRS_LIST.0 "next wraps at end of list"

    dirs prev
    assert equal $c.base_path $env.DIRS_LIST.0 "prev wraps at top of list"

    # Stack becomes: [base PATH_B path_a]
    dirs add $c.path_b $c.path_a
    assert equal $c.path_b $env.PWD "add changes PWD to first added dir"
    assert length $env.DIRS_LIST 3 "add in fact adds to list"
    assert equal $c.path_a $env.DIRS_LIST.2 "add in fact adds to list"

    # Stack becomes: [BASE path_b path_a]
    dirs next 2
    # assert (not) equal requires span.start of first arg < span.end of 2nd
    assert equal $env.PWD $c.base_path "next wraps at end of list"

    # Stack becomes: [base path_b PATH_A]
    dirs prev 1
    assert equal $c.path_a $env.PWD "prev wraps at start of list"
    cur_dir_check $c.path_a "prev wraps to end from start of list"

    # Stack becomes: [base PATH_B]
    dirs drop
    assert length $env.DIRS_LIST 2 "drop removes from list"
    assert equal $env.PWD $c.path_b "drop changes PWD to previous in list (before dropped element)"

    assert equal (dirs) [[active path]; [false $c.base_path] [true $c.path_b]] "show table contains expected information"

    # Stack becomes: [BASE]
    dirs drop
    assert length $env.DIRS_LIST 1 "drop removes from list"
    assert equal $env.PWD $c.base_path "drop changes PWD (regression test for #9449)"
}

@test
def dirs_next [] {
    # must capture value of $in before executing `use`s
    let $c = $in
    # must set PWD *before* doing `use` that will run the def --env block in dirs module.
    cd $c.base_path
    assert equal $env.PWD $c.base_path "test setup"

    hide-env -i DIRS_LIST DIRS_POSITION
    use std/dirs
    cur_dir_check $c.base_path "use module test setup"

    dirs add $c.path_a $c.path_b
    cur_ring_check $c.path_a 1 "add 2, but pwd is first one added"

    dirs next
    cur_ring_check $c.path_b 2 "next to third"

    dirs next
    cur_ring_check $c.base_path 0 "next from top wraps to bottom of ring"

}

@test
def dirs_cd [] {
    # must capture value of $in before executing `use`s
    let $c = $in
    # must set PWD *before* doing `use` that will run the def --env block in dirs module.
    cd $c.base_path

    hide-env -i DIRS_LIST DIRS_POSITION
    use std/dirs

    cur_dir_check $c.base_path "use module test setup"

    cd $c.path_b
    cur_ring_check $c.path_b 0 "cd with empty ring"

    dirs add $c.path_a
    cur_dir_check $c.path_a "can add 2nd directory"

    cd $c.path_b
    cur_ring_check $c.path_b 1 "cd at 2nd item on ring"

    dirs next
    cur_ring_check $c.path_b 0 "cd updates current position in non-empty ring"
    assert equal [$c.path_b $c.path_b] $env.DIRS_LIST "cd updated both positions in ring"
}

@test
def dirs_goto_bug10696 [] {
    let $c = $in
    cd $c.base_path

    hide-env -i DIRS_LIST DIRS_POSITION
    use std/dirs

    dirs add $c.path_a
    cd $c.path_b
    dirs goto 0
    dirs goto 1

    assert equal $env.PWD $c.path_b "goto other, then goto to come back returns to same directory"
}

@test
def dirs_goto [] {
    let $c = $in
    cd $c.base_path

    hide-env -i DIRS_LIST DIRS_POSITION
    use std/dirs

    # check that goto can move *from* any position in the ring *to* any other position (correctly)

    assert equal $env.PWD $c.base_path
    dirs add $c.path_a
    dirs add $c.path_b
    assert equal ($env.DIRS_LIST | length) 3 "start with 3 elements in ring"
    
    let exp_dir = [$c.base_path $c.path_a $c.path_b]
    
    for $cur_pos in 0..<($env.DIRS_LIST | length) {
        for $other_pos in 0..<($env.DIRS_LIST | length) {
            dirs goto $cur_pos
            assert equal $env.PWD ($exp_dir | get $cur_pos) "initial position as expected"

            dirs goto $other_pos
            assert equal $env.DIRS_POSITION $other_pos "goto moved index to correct slot"
            assert equal $env.PWD ($exp_dir | get $other_pos) "goto changed working directory correctly"
        }
    }

    # check that 'dirs goto' with no argument maps to `dirs` (main)
    assert length (dirs goto) 3
}
