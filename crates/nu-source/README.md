# nu-source

## Overview

Inside this crate, you will find the types and behaviors responsible for metadata inside `Nushell` with additional debugging features.

Important Types

### Text
Represent the value of an input file. 
Similar to a `String` in rust, but cheaply cloneable.

### AnchorLocation
An enum that represents the location where a value originated from. 
ex. Url, File, or Source `Text`

### Span
A start point and end point

### Spanned
A _spanned_ value. This combines a `Span` with some value.

### Tag
Metadata that can be associated with some value.
A tag is made from an `AnchorLocation` and `Span`.
Tags can be made with unknown locations or spans.

### Tagged
A _tagged_ value. This combines a `Tag` with some value.
