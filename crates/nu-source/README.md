# nu-source

## Overview

The `nu-source` crate contains types and traits used for keeping track of _metadata_ about values being processed. 
Nu uses `Tag`s to keep track of where a value came from, an `AnchorLocation`,
as well as positional information about the value, a `Span`.
An `AchorLocation` can be a `Url`, `File`, or `Source` text that a value was parsed from.
The source `Text` is special in that it is a type similar to a `String` that comes with the ability to be cheaply cloned.
A `Span` keeps track of a value's `start` and `end` positions.
These types make up the metadata for a value and are wrapped up together in a `Tagged` struct,
which holds everything needed to track and locate a value. 

In addition to metadata tracking, `nu-source` also contains types and traits related to debugging, tracing, and formatting the metadata and values it processes.