use std-rfc/iter recurse

def children []: list<record> -> list<record> {
	where ($it.content | describe --detailed).type == list
	| get content
	| flatten
}

def descendant-or-self []: list<record<content: list<record>>> -> list<record>  {
	recurse {
		where ($it.content | describe --detailed).type == list
		| get content
	}
	| get item
	| flatten
}

export def pipeline [meta: record]: list<oneof<cell-path, string, int, closure, list>> -> closure {
	let steps = each {|e|
		if ($e | describe) == "cell-path" {
			$e | split cell-path | get value
		} else {
			$e | prepend null  # make sure it's a list so `flatten` behaves in a predictable manner
		}
	}
	| flatten

	if ($steps | is-empty) {
		error make {
			msg: 'Empty path provided'
			label: {
				text: 'Use a non-empty list of path steps'
				span: $meta.span
			}
		}
	}

	$steps
	| reduce --fold {|| } {|step, prev|
		match ($step | describe) {
			"string" => {
				match $step {
					"*" => {|| do $prev | children }
					"**" => {|| do $prev | descendant-or-self }
					$tag => {|| do $prev | children | where tag == $tag }
				}
			}
			"int" => {|| do $prev | select $step }
			"closure" => {|| do $prev | where $step }
			$type => {
				let step_span = (metadata $step).span
				error make {
					msg: $'Incorrect path step type ($type)'
					label: {
						text: 'Use a string or int as a step'
						span: $step_span
					}
				}
			}
		}
	}
}

export def xaccess [...rest: oneof<cell-path, closure, list>] {
	[{content: ($in | prepend null)}]
	| do ($rest | pipeline (metadata $rest))
}
