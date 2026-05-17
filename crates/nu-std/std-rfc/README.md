# `std-rfc`

## Overview and Requirements

This module includes potential candidate commands (and other definitions) for inclusion in the Standard Library (`std`) that is built in to Nushell. As a general guideline, candidates should:

* Be general purpose features that will be useful to a number of users
* Include doc comments for definitions and parameters that can be used with `help <command>`
* Include tests
* Since doc comments are fairly limited, additional documentation can be included
  in a GitHub discussion. This documentation can then be moved to the main website when the feature
  is promoted to `std`. See [this example](https://github.com/nushell/nushell/discussions/14935#discussion-7882769) for some `table` helpers.

## Showcase and Discussion

While primary feedback should take place in the PR, we have also established a [Drawing Board](https://discord.com/channels/601130461678272522/1313988919477538888) Discord channel which can be used for several purposes:

* Ideation before a PR is submitted
* Raise awareness of the feature
* Short-term questions and discussion

Note: The Drawing Board is not just for `std-rfc`. Please tag your topic with `std-library` if it is about a Standard Library idea.

## Promotion Evaluation

In general, new `std-rfc` features will be evaluated after they have been trialed for a sufficient period, which may vary depending on the feature. After that period, the feature may be:

* Promoted to `std`
* Removed from `std-rfc`
* Or changes may be requested and then reevaluated later
