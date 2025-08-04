# for examples
alias "random choice" = choice

# Sample `k` elements from a list
#
# This function will pick a simple random sample from input without replacement
# (each element from the input can only be picked once).
#
# The sample is treated as a set.  This means that the combined probability of
# `[1 2 3 4] | random choice 2` returning `[3, 4]` or `[4, 3]` equals that of
# `[1, 2]`.  To ensure that all permutations are equally probable, use
# `shuffle` or `sort`.
#
# The current implementation collects the input stream.  This might change in
# the future.
@example "Pick 2 random items" {
	[1 2 3 4 5] | random choice 2
}
@example "Verify that the elements are picked uniformly" {
	0..100_000
	| each {
		[1 2 3 4 5] | random choice 2 | sort | to nuon
	}
	| histogram
}
export def choice [
	n: int = 1  # number of items to sample
]: list -> list {
	# XXX: this collects the stream
	let input = $in

	let len = $input | length
	if $n > ($input | length) {
		error make {
			msg: "Can't sample more elements than there are in input"
			label: {
				text: $"Tried to sample ($n) out of ($len)"
				span: (metadata $n).span
			}
		}
	}

	# always return a list, even though `first 1` returns standalone T
	mut output = $input | if $n == 1 {
		first | [$in]
	} else {
		first $n
	}

	# reservoir sampling, algorithm L
	# https://doi.org/10.1145/198429.198435

	mut w = (random float) ** (1 / $n)
	mut i = $n - 1

	loop {
		$i += (random float | math ln) / (1.0 - $w | math ln)
			| math floor
			| $in + 1

		if $i < $len {
			let el = $input | get $i
			$output = $output | update (random int 0..<$n) $el

			$w *= (random float) ** (1 / $n)
		} else {
			break
		}
	}

	$output
}
