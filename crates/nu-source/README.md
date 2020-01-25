# nu-source

## Overview

Inside this crate you will find the types and traits responsible for keeping track
The `nu-source` crate contains types and traits that nu uses to keep track of the values it is processing. This type of data is referred to as "metadata" and there are several types of information that is tracked. Inside Nu, values are `Tagged`, which is a data structure that keeps track of the item and its metadata, which is also known as a `Tag`. A `Tag` is made up of location based information such as an `AnchorLocation` as well as a `Span`. An `AchorLocation` represents the location where a value originated from. This can be a `Url`, `File`, or `Source` text that a value was parsed from.
The source `Text` is special in that it is a type similar to a `String` with the ability to be cheaply cloned.
A `Span` is used to keep track of the position of a value with a `start` and `end`.
