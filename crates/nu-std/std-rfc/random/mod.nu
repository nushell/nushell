# for examples
alias "random choice" = choice

# Sample `k` elements from a list
#
# The function will return a list the length of `k` by randomly sampling
# elements from the input.  Each element can only be picked once, all with the
# same probability.  The order of the elements is also random: even if `a`
# comes before `b` input, `random choice 2` can return `[b a]`.
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
